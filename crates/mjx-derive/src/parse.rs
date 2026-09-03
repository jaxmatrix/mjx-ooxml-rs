//! Parsing `#[derive(FromXml, ToXml)]` input + the `#[xml(..)]` attribute grammar into an IR.

use proc_macro2::Span;
use syn::{
    spanned::Spanned, Data, DeriveInput, Expr, Fields, GenericArgument, Generics, Ident, LitStr,
    Path, PathArguments, Type, Visibility,
};

use crate::naming::snake_case;

/// The parsed shape of a derivable type.
pub(crate) enum XmlType {
    /// A struct with a `#[xml(children)]` field: framework fields + an ordered content `Vec`.
    Container(Container),
    /// A struct with a `#[xml(text)]` field: framework fields + a decoded `String`.
    TextLeaf(TextLeaf),
}

/// A container type (`a:txBody` / `a:p` / `a:r`).
pub(crate) struct Container {
    pub self_ty: Ident,
    pub generics: Generics,
    /// The name of the content field (usually `content`).
    pub content_field: Ident,
    /// The content enum type, e.g. `TextBodyContent` (stripped from `Vec<..>`).
    pub enum_path: Path,
    /// The catch-all variant name (always `Raw`).
    pub raw_variant: Ident,
    /// The declared typed children, in order.
    pub children: Vec<ChildSpec>,
}

/// One `child(local = .., variant = .., ty = .., ns = ..)` entry.
pub(crate) struct ChildSpec {
    pub local: LitStr,
    /// The fully-qualified path to the `SchemaNamespace` constant (already resolved).
    pub namespace: Path,
    pub variant: Ident,
    pub ty: Path,
}

/// The typed attributes declared on one struct — what `#[derive(XmlAttributes)]` expands.
pub(crate) struct AttributeModel {
    pub self_ty: Ident,
    pub generics: Generics,
    /// The struct's own visibility: the generated accessors match it, so a private type does not
    /// grow a `pub fn` (which `unreachable_pub` would rightly complain about).
    pub vis: Visibility,
    /// The declared type of the `attributes` field.
    ///
    /// Kept because the generated accessors reach the vector through `AsRef`/`AsMut` rather than
    /// through the field's concrete type, and the bound that makes that legal has to name this type.
    /// It is what lets one declaration serve a type that *owns* its attributes and a view that only
    /// borrows them.
    pub attributes_ty: Type,
    pub attributes: Vec<AttributeSpec>,
}

/// One `attribute(local = .., codec = .., ..)` entry.
pub(crate) struct AttributeSpec {
    /// The exact wire local name.
    pub local: LitStr,
    /// The literal prefix to match and to write, or `None` for an unprefixed attribute.
    pub prefix: Option<LitStr>,
    /// The `AttributeCodec` implementor. A `Type`, not a `Path`, so `Enumeration<LineCap>` works.
    pub codec: Type,
    pub getter: Ident,
    pub setter: Ident,
    /// `prefix:local`, or `local` — what errors name and what the docs quote.
    pub qualified: String,
    pub presence: Presence,
    pub span: Span,
}

/// Whether an attribute must be there, may be absent, or has a schema default when absent.
pub(crate) enum Presence {
    /// Absent is [`AttributeError::Missing`], never a substituted value.
    Required,
    /// Absent reads as `None`.
    Optional,
    /// Absent reads as the schema's default. Boxed: an `Expr` is large and this variant is rare.
    Defaulted(Box<Expr>),
}

impl AttributeModel {
    /// Parses a `#[derive(XmlAttributes)]` input into the IR.
    ///
    /// Deliberately looser than [`XmlType::from_derive_input`]: it asks only for the retained
    /// `attributes` vector, so the derive composes with `#[derive(FromXml, ToXml)]`, with a
    /// hand-written pair of impls, and with a fidelity wrapper that keeps its children as raw nodes.
    pub(crate) fn from_derive_input(input: &DeriveInput) -> syn::Result<Self> {
        let Data::Struct(data) = &input.data else {
            return Err(syn::Error::new_spanned(
                input,
                "XmlAttributes can only be derived for structs",
            ));
        };
        let Fields::Named(fields) = &data.fields else {
            return Err(syn::Error::new_spanned(
                &data.fields,
                "XmlAttributes requires a struct with named fields",
            ));
        };
        let Some(attributes_field) = fields.named.iter().find(|field| {
            field
                .ident
                .as_ref()
                .is_some_and(|ident| ident == "attributes")
        }) else {
            return Err(syn::Error::new_spanned(
                input,
                "XmlAttributes generates accessors over the retained `attributes: Vec<RawAttribute>` \
                 field, which this type does not have",
            ));
        };
        let attributes_ty = attributes_field.ty.clone();

        let (_namespace, attributes) = parse_struct_xml(input)?;
        if attributes.is_empty() {
            return Err(syn::Error::new_spanned(
                input,
                "XmlAttributes needs at least one struct-level `#[xml(attribute(local = .., codec = ..))]`",
            ));
        }

        Ok(Self {
            self_ty: input.ident.clone(),
            generics: input.generics.clone(),
            vis: input.vis.clone(),
            attributes_ty,
            attributes,
        })
    }
}

/// A text-leaf type (`a:t`).
pub(crate) struct TextLeaf {
    pub self_ty: Ident,
    pub generics: Generics,
    /// The name of the `#[xml(text)]` field (usually `text`).
    pub text_field: Ident,
}

/// A field's parsed `#[xml(..)]` role.
enum ContentKind {
    Children {
        enum_path: Path,
        children: Vec<ChildSpec>,
    },
    Text,
}

impl XmlType {
    /// Parses a `#[derive(FromXml, ToXml)]` input into the IR, or a `syn::Error` to be turned into a
    /// `compile_error!`.
    pub(crate) fn from_derive_input(input: &DeriveInput) -> syn::Result<Self> {
        let Data::Struct(data) = &input.data else {
            return Err(syn::Error::new_spanned(
                input,
                "FromXml/ToXml can only be derived for structs",
            ));
        };
        let Fields::Named(fields) = &data.fields else {
            return Err(syn::Error::new_spanned(
                &data.fields,
                "FromXml/ToXml requires a struct with named fields",
            ));
        };

        let (namespace_default, _attributes) = parse_struct_xml(input)?;

        let (mut has_name, mut has_attributes, mut has_empty) = (false, false, false);
        let mut content: Option<(Ident, ContentKind)> = None;

        for field in &fields.named {
            let ident = field
                .ident
                .clone()
                .expect("named field has an ident by construction");
            match parse_field_xml(field, namespace_default.as_ref())? {
                Some(kind) => {
                    if content.is_some() {
                        return Err(syn::Error::new_spanned(
                            field,
                            "expected exactly one `#[xml(children)]` or `#[xml(text)]` field",
                        ));
                    }
                    content = Some((ident, kind));
                }
                None => match ident.to_string().as_str() {
                    "name" => has_name = true,
                    "attributes" => has_attributes = true,
                    "empty" => has_empty = true,
                    _ => {
                        return Err(syn::Error::new_spanned(
                            field,
                            "unexpected field: a derivable type has exactly `name`, `attributes`, \
                             `empty`, and one `#[xml(children)]`/`#[xml(text)]` content field",
                        ));
                    }
                },
            }
        }

        for (present, missing) in [
            (has_name, "name: RawName"),
            (has_attributes, "attributes: Vec<RawAttribute>"),
            (has_empty, "empty: bool"),
        ] {
            if !present {
                return Err(syn::Error::new_spanned(
                    input,
                    format!("derivable type is missing the required field `{missing}`"),
                ));
            }
        }

        let Some((content_field, kind)) = content else {
            return Err(syn::Error::new_spanned(
                input,
                "derivable type needs one `#[xml(children)]` or `#[xml(text)]` content field",
            ));
        };

        Ok(match kind {
            ContentKind::Children {
                enum_path,
                children,
            } => XmlType::Container(Container {
                self_ty: input.ident.clone(),
                generics: input.generics.clone(),
                content_field,
                enum_path,
                raw_variant: Ident::new("Raw", Span::call_site()),
                children,
            }),
            ContentKind::Text => XmlType::TextLeaf(TextLeaf {
                self_ty: input.ident.clone(),
                generics: input.generics.clone(),
                text_field: content_field,
            }),
        })
    }
}

/// Parses the struct-level `#[xml(..)]` options: the default child namespace, and the declared
/// typed attributes.
///
/// One scanner for both, so that `#[derive(FromXml)]` and `#[derive(XmlAttributes)]` on the same
/// struct agree about what its `#[xml(..)]` attributes mean and neither rejects the other's keys.
fn parse_struct_xml(input: &DeriveInput) -> syn::Result<(Option<Path>, Vec<AttributeSpec>)> {
    let mut namespace = None;
    let mut attributes: Vec<AttributeSpec> = Vec::new();
    for attr in &input.attrs {
        if !attr.path().is_ident("xml") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("namespace") {
                let path: Path = meta.value()?.parse()?;
                namespace = Some(qualify_namespace(&path));
                Ok(())
            } else if meta.path.is_ident("attribute") {
                attributes.push(parse_attribute(&meta)?);
                Ok(())
            } else {
                Err(meta.error(
                    "unknown `#[xml(..)]` option on struct (expected `namespace` or `attribute(..)`)",
                ))
            }
        })?;
    }

    for (index, spec) in attributes.iter().enumerate() {
        if let Some(earlier) = attributes[..index]
            .iter()
            .find(|other| other.qualified == spec.qualified)
        {
            return Err(syn::Error::new(
                spec.span,
                format!(
                    "`{}` is declared twice; an element cannot carry the same attribute twice",
                    earlier.qualified
                ),
            ));
        }
        if let Some(earlier) = attributes[..index]
            .iter()
            .find(|other| other.getter == spec.getter)
        {
            return Err(syn::Error::new(
                spec.span,
                format!(
                    "`{}` and `{}` would both generate the accessor `{}`; set `accessor = ..` on one \
                     of them",
                    earlier.qualified, spec.qualified, spec.getter
                ),
            ));
        }
    }

    Ok((namespace, attributes))
}

/// Parses one `attribute(local = .., codec = .., prefix = .., accessor = .., required, default = ..)`.
fn parse_attribute(meta: &syn::meta::ParseNestedMeta<'_>) -> syn::Result<AttributeSpec> {
    let mut local: Option<LitStr> = None;
    let mut prefix: Option<LitStr> = None;
    let mut codec: Option<Type> = None;
    let mut accessor: Option<Ident> = None;
    let mut required = false;
    let mut default: Option<Expr> = None;

    meta.parse_nested_meta(|inner| {
        if inner.path.is_ident("local") {
            local = Some(inner.value()?.parse()?);
        } else if inner.path.is_ident("prefix") {
            prefix = Some(inner.value()?.parse()?);
        } else if inner.path.is_ident("codec") {
            codec = Some(inner.value()?.parse()?);
        } else if inner.path.is_ident("accessor") {
            accessor = Some(inner.value()?.parse()?);
        } else if inner.path.is_ident("required") {
            required = true;
        } else if inner.path.is_ident("default") {
            default = Some(inner.value()?.parse()?);
        } else {
            return Err(inner.error(
                "unknown `attribute(..)` key (expected `local`, `codec`, `prefix`, `accessor`, \
                 `required`, `default`)",
            ));
        }
        Ok(())
    })?;

    let span = meta.path.span();
    let local = local.ok_or_else(|| syn::Error::new(span, "`attribute(..)` is missing `local`"))?;
    let codec = codec.ok_or_else(|| {
        syn::Error::new(
            span,
            "`attribute(..)` is missing `codec` — the `AttributeCodec` that decodes its values, e.g. \
             `codec = ::mjx_ooxml_types::OnOff`",
        )
    })?;

    let presence =
        match (required, default) {
            (true, Some(_)) => return Err(syn::Error::new(
                span,
                "`required` and `default` contradict each other: a required attribute is always \
                 present, so no default can apply",
            )),
            (true, None) => Presence::Required,
            (false, Some(expr)) => Presence::Defaulted(Box::new(expr)),
            (false, None) => Presence::Optional,
        };

    let qualified = match &prefix {
        Some(prefix) => format!("{}:{}", prefix.value(), local.value()),
        None => local.value(),
    };
    let getter = match accessor {
        Some(ident) => ident,
        None => Ident::new(&snake_case(&local.value()), local.span()),
    };
    let setter = Ident::new(&format!("set_{getter}"), getter.span());

    Ok(AttributeSpec {
        local,
        prefix,
        codec,
        getter,
        setter,
        qualified,
        presence,
        span,
    })
}

/// Parses a field's `#[xml(..)]`, returning its content role (or `None` for a framework field).
fn parse_field_xml(
    field: &syn::Field,
    namespace_default: Option<&Path>,
) -> syn::Result<Option<ContentKind>> {
    let Some(attr) = field.attrs.iter().find(|a| a.path().is_ident("xml")) else {
        return Ok(None);
    };

    let mut is_children = false;
    let mut is_text = false;
    let mut children: Vec<ChildSpec> = Vec::new();

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("children") {
            is_children = true;
            Ok(())
        } else if meta.path.is_ident("text") {
            is_text = true;
            Ok(())
        } else if meta.path.is_ident("child") {
            children.push(parse_child(&meta, namespace_default)?);
            Ok(())
        } else {
            Err(meta.error("unknown `#[xml(..)]` option (expected `children`, `text`, or `child`)"))
        }
    })?;

    match (is_children, is_text) {
        (true, true) => Err(syn::Error::new_spanned(
            attr,
            "a field cannot be both `#[xml(children)]` and `#[xml(text)]`",
        )),
        (false, false) => Err(syn::Error::new_spanned(
            attr,
            "expected `#[xml(children, ..)]` or `#[xml(text)]`",
        )),
        (true, false) => {
            if children.is_empty() {
                return Err(syn::Error::new_spanned(
                    attr,
                    "`#[xml(children)]` needs at least one `child(..)`",
                ));
            }
            let enum_path = enum_path_from_vec(&field.ty)?;
            Ok(Some(ContentKind::Children {
                enum_path,
                children,
            }))
        }
        (false, true) => {
            if !children.is_empty() {
                return Err(syn::Error::new_spanned(
                    attr,
                    "`#[xml(text)]` does not take `child(..)`",
                ));
            }
            Ok(Some(ContentKind::Text))
        }
    }
}

/// Parses one `child(local = .., variant = .., ty = .., ns = ..)`.
fn parse_child(
    meta: &syn::meta::ParseNestedMeta<'_>,
    namespace_default: Option<&Path>,
) -> syn::Result<ChildSpec> {
    let mut local: Option<LitStr> = None;
    let mut variant: Option<Ident> = None;
    let mut ty: Option<Path> = None;
    let mut namespace: Option<Path> = None;

    meta.parse_nested_meta(|inner| {
        if inner.path.is_ident("local") {
            local = Some(inner.value()?.parse()?);
        } else if inner.path.is_ident("variant") {
            variant = Some(inner.value()?.parse()?);
        } else if inner.path.is_ident("ty") {
            ty = Some(inner.value()?.parse()?);
        } else if inner.path.is_ident("ns") {
            let path: Path = inner.value()?.parse()?;
            namespace = Some(qualify_namespace(&path));
        } else {
            return Err(
                inner.error("unknown `child(..)` key (expected `local`, `variant`, `ty`, `ns`)")
            );
        }
        Ok(())
    })?;

    let span = meta.path.span();
    let local = local.ok_or_else(|| syn::Error::new(span, "`child(..)` is missing `local`"))?;
    let variant =
        variant.ok_or_else(|| syn::Error::new(span, "`child(..)` is missing `variant`"))?;
    let ty = ty.ok_or_else(|| syn::Error::new(span, "`child(..)` is missing `ty`"))?;
    let namespace = namespace
        .or_else(|| namespace_default.cloned())
        .ok_or_else(|| {
            syn::Error::new(
                span,
                "`child(..)` needs a namespace: set `ns = ..` or a struct-level `#[xml(namespace = ..)]`",
            )
        })?;

    Ok(ChildSpec {
        local,
        namespace,
        variant,
        ty,
    })
}

/// Resolves a namespace reference: a bare ident (`DML_MAIN`) becomes
/// `::mjx_ooxml_types::namespaces::DML_MAIN`; any multi-segment or rooted path is used verbatim.
fn qualify_namespace(path: &Path) -> Path {
    if path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments[0].arguments.is_none()
    {
        let ident = &path.segments[0].ident;
        syn::parse_quote!(::mjx_ooxml_types::namespaces::#ident)
    } else {
        path.clone()
    }
}

/// Extracts `T` from a `Vec<T>` field type (the content enum path).
fn enum_path_from_vec(ty: &Type) -> syn::Result<Path> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(Type::Path(inner))) = args.args.first() {
                        return Ok(inner.path.clone());
                    }
                }
            }
        }
    }
    Err(syn::Error::new_spanned(
        ty,
        "a `#[xml(children)]` field must have type `Vec<SomeContentEnum>`",
    ))
}

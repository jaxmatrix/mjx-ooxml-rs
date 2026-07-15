//! Parsing `#[derive(FromXml, ToXml)]` input + the `#[xml(..)]` attribute grammar into an IR.

use proc_macro2::Span;
use syn::{
    spanned::Spanned, Data, DeriveInput, Fields, GenericArgument, Generics, Ident, LitStr, Path,
    PathArguments, Type,
};

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

        let namespace_default = parse_struct_namespace(input)?;

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

/// Parses a struct-level `#[xml(namespace = <path>)]`, if any.
fn parse_struct_namespace(input: &DeriveInput) -> syn::Result<Option<Path>> {
    let mut namespace = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("xml") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("namespace") {
                let path: Path = meta.value()?.parse()?;
                namespace = Some(qualify_namespace(&path));
                Ok(())
            } else {
                Err(meta.error("unknown `#[xml(..)]` option on struct (expected `namespace`)"))
            }
        })?;
    }
    Ok(namespace)
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

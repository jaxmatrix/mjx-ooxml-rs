/**
 * `mjx/story-conventions` — a story is not ready for audit without its three declarations.
 *
 * `BUILD_PLAN_LOOP_1.md` §3 requires every component to ship a states matrix, a token-dependency
 * list, and its keyboard and screen-reader behaviour. `src/story/conventions.ts` validates those at
 * runtime, which catches an *empty* list and a *stale* token name. It cannot catch the failure mode
 * that actually matters across fifteen children: a story file that never calls `storyConventions`
 * at all and exports a bare Storybook meta, which runs perfectly and declares nothing.
 *
 * So the static half is this rule, and the division of labour is deliberate:
 *
 * | Failure | Caught by |
 * |---|---|
 * | Story exports a meta with no conventions in it | this rule |
 * | A required declaration is absent | this rule |
 * | A declaration is present but empty | this rule, and again at runtime |
 * | A token dependency names a token that no longer exists | runtime, against the generated table |
 *
 * ## The shape it looks for
 *
 * Storybook's indexer reads CSF **statically** and refuses a default export that is not an object
 * literal, so the conventions cannot be attached by wrapping the meta. They live in the meta's
 * `parameters.mjx` instead:
 *
 * ```ts
 * const meta: Meta = {
 *   title: 'Gates/Contrast',
 *   parameters: { mjx: storyConventions({ statesMatrix: [...], … }) },
 * };
 * export default meta;
 * ```
 *
 * The rule is proved able to fail in `tests/story-conventions.test.ts`, which runs ESLint over
 * deliberately bad sources. A lint rule nobody has watched fail is a lint rule that might be
 * matching nothing.
 */

/** The declarations a story's conventions must carry. */
const required = ['statesMatrix', 'tokenDependencies', 'keyboard', 'screenReader'];

/** The ones that are lists, and that an empty literal makes vacuous. */
const mustBeNonEmptyArrays = ['statesMatrix', 'tokenDependencies', 'keyboard'];

/** The function that validates them, and the parameter key they live under. */
const factory = 'storyConventions';
const parameterKey = 'mjx';

/**
 * @param {import('eslint').Rule.RuleContext} context
 * @param {any} node
 * @returns {any}
 */
function unwrap(context, node) {
  if (node === null || node === undefined) return null;
  if (node.type === 'TSSatisfiesExpression' || node.type === 'TSAsExpression') {
    return unwrap(context, node.expression);
  }
  if (node.type === 'ObjectExpression') return node;
  if (node.type === 'Identifier') {
    const scope = context.sourceCode.getScope(node);
    let variable = null;
    for (let cursor = scope; cursor !== null && variable === null; cursor = cursor.upper) {
      variable = cursor.variables.find((candidate) => candidate.name === node.name) ?? null;
    }
    const definition = variable?.defs?.[0];
    if (definition !== undefined && definition.node.type === 'VariableDeclarator') {
      return unwrap(context, definition.node.init);
    }
  }
  return null;
}

/**
 * Follow identifiers and type assertions until a call expression is reached, or give up.
 *
 * @param {import('eslint').Rule.RuleContext} context
 * @param {any} node
 * @returns {any}
 */
function resolveCall(context, node) {
  if (node === null || node === undefined) return null;
  if (node.type === 'CallExpression') return node;
  if (node.type === 'TSSatisfiesExpression' || node.type === 'TSAsExpression') {
    return resolveCall(context, node.expression);
  }
  if (node.type === 'Identifier') {
    const scope = context.sourceCode.getScope(node);
    let variable = null;
    for (let cursor = scope; cursor !== null && variable === null; cursor = cursor.upper) {
      variable = cursor.variables.find((candidate) => candidate.name === node.name) ?? null;
    }
    const definition = variable?.defs?.[0];
    if (definition !== undefined && definition.node.type === 'VariableDeclarator') {
      return resolveCall(context, definition.node.init);
    }
  }
  return null;
}

/**
 * @param {any} object
 * @param {string} name
 * @returns {any}
 */
function property(object, name) {
  return (
    object.properties.find(
      (entry) =>
        entry.type === 'Property' &&
        ((entry.key.type === 'Identifier' && entry.key.name === name) ||
          (entry.key.type === 'Literal' && entry.key.value === name)),
    ) ?? null
  );
}

/** @type {import('eslint').Rule.RuleModule} */
const rule = {
  meta: {
    type: 'problem',
    docs: {
      description:
        'A story must declare its states matrix, its token dependencies and its keyboard and ' +
        'screen-reader behaviour, through storyConventions() in its meta parameters.',
    },
    schema: [],
    messages: {
      noDefaultExport:
        'A story file must default-export its meta. Without one the catalogue cannot index it and ' +
        'this rule cannot check it.',
      unresolvable:
        'This rule cannot see the story meta. Default-export an object literal, or a const ' +
        'initialised with one — which is also what Storybook’s indexer requires.',
      noParameters:
        "A story meta must carry `parameters.{{key}}`. BUILD_PLAN_LOOP_1.md §3: a component " +
        'without its states matrix, token dependencies and keyboard/screen-reader behaviour is ' +
        'not ready for audit.',
      notThroughFactory:
        'parameters.{{key}} must be built with {{factory}}() so its token dependencies are checked ' +
        'against the generated token table. A bare object declares the list without validating it.',
      missing:
        "A story's conventions must declare '{{name}}'.",
      empty:
        "'{{name}}' is empty. An empty declaration is a declaration nobody wrote — say what the " +
        'single row is instead.',
    },
  },

  create(context) {
    return {
      Program(program) {
        const exported = program.body.find(
          (statement) => statement.type === 'ExportDefaultDeclaration',
        );
        if (exported === undefined) {
          context.report({ node: program, messageId: 'noDefaultExport' });
          return;
        }

        const meta = unwrap(context, /** @type {any} */ (exported).declaration);
        if (meta === null) {
          context.report({ node: exported, messageId: 'unresolvable' });
          return;
        }

        const parameters = property(meta, 'parameters');
        const parametersObject =
          parameters === null ? null : unwrap(context, parameters.value);
        const declaration =
          parametersObject === null ? null : property(parametersObject, parameterKey);

        if (declaration === null) {
          context.report({
            node: meta,
            messageId: 'noParameters',
            data: { key: parameterKey },
          });
          return;
        }

        // Follow an identifier to whatever it was initialised with, so a story may hoist its
        // conventions into a named const if that reads better than nesting them.
        const call = resolveCall(context, declaration.value);
        const throughFactory =
          call !== null &&
          call.type === 'CallExpression' &&
          call.callee.type === 'Identifier' &&
          call.callee.name === factory;
        if (!throughFactory) {
          context.report({
            node: declaration,
            messageId: 'notThroughFactory',
            data: { key: parameterKey, factory },
          });
        }

        const conventions =
          call !== null && call.type === 'CallExpression'
            ? unwrap(context, call.arguments[0])
            : unwrap(context, declaration.value);
        if (conventions === null) {
          context.report({ node: declaration, messageId: 'unresolvable' });
          return;
        }

        for (const name of required) {
          const found = property(conventions, name);
          if (found === null) {
            context.report({ node: conventions, messageId: 'missing', data: { name } });
            continue;
          }
          if (
            mustBeNonEmptyArrays.includes(name) &&
            found.value.type === 'ArrayExpression' &&
            found.value.elements.length === 0
          ) {
            context.report({ node: found, messageId: 'empty', data: { name } });
          }
        }
      },
    };
  },
};

export default { rules: { 'story-conventions': rule } };

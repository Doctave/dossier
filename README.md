# Dossier, a multi-language docstrings parser

Dossier reads source code and generates JSON that describes the elements in the code it finds. It is built on the fantastic [tree-sitter](https://tree-sitter.github.io/tree-sitter/) library, and supports multiple languages.

The goal is to have one single tool and schema for analysing any kind of source code.

The JSON output can be used for example to:

- Generate HTML documentation
- Analyse your source code
- Run checks in CI/CD to verify aspects of your source code

This project is maintained by [Doctave](https://www.doctave.com).

## Example Usage

Here's an example that parses an interface in the popular [kysely](https://kysely.dev/) library.

````
$ dossier kysely/src/expression/expression.ts
[
  {
    "title": "Expression<T>",
    "description": "`Expression` represents an arbitrary SQL expression with a type.\n\nMost Kysely methods accept instances of `Expression` and most classes like `SelectQueryBuilder`\nand the return value of the {@link sql} template tag implement it.\n\n```ts\nconst exp1: Expression<string> = sql<string>`CONCAT('hello', ' ', 'world')`\nconst exp2: Expression<{ first_name: string }> = db.selectFrom('person').select('first_name')\n```\n\nYou can implement the `Expression` interface to create your own type-safe utilities for Kysely.",
    "kind": "interface",
    "children": [
      {
        "title": "expressionType",
        "description": "/**\n   * All expressions need to have this getter for complicated type-related reasons.\n   * Simply add this getter for your expression and always return `undefined` from it:\n   *\n   * ```ts\n   * class SomeExpression<T> implements Expression<T> {\n   *   get expressionType(): T | undefined {\n   *     return undefined\n   *   }\n   * }\n   * ```\n   *\n   * The getter is needed to make the expression assignable to another expression only\n   * if the types `T` are assignable. Without this property (or some other property\n   * that references `T`), you could assing `Expression<string>` to `Expression<number>`.\n   */",
        "kind": "method",
        "children": [],
        "language": "ts",
        "source": {
          "file": "fixtures/kysely/src/expression/expression.ts",
          "start_offset_bytes": 1525,
          "end_offset_bytes": 1560,
          "repository": null
        },
        "meta": {
          "return_type": "T | undefined"
        }
      },
 ...
````

## How it works

Dossier uses [`tree-sitter`](https://tree-sitter.github.io/tree-sitter/) to parse source code. When implementing a new language in Dossier, the author uses tree-sitter [queries](https://tree-sitter.github.io/tree-sitter/using-parsers#pattern-matching-with-queries) to find the relevant language features: e.g. methods, classes, interfaces, etc. These features are then converted into a standardized output format.

## The `Entity` data structure

The `Entity` data structure is the foundation of Dossier: every element or "entity" found in any language is described by this object.

For this reason, a lot of thought has gone into designing this structure. It has to be flexible enought to work in languages with different features, but also concise enough to be easily understandable so it can be integrated into other tools.

### The basics

Let's take a typescript function like this:

```typescript
/**
 * Docs for this function
 */
function parse(input: string, config: ParserConfig): Ast {
  // ...
}
```

How would we convert this into Dossier entities?

Well, first of all, we'd have a top level `function` entity, with the `title` of `parse`.

```json
// Some fields removed for brevity
{
  "title": "parse",
  "kind": "function",
  "description": "Docs for this function"
}
```

But how do we handle the parameters and return type?

Instead of having `return_type` or `parameters` fields on an entity, we have a general `members` list. Any entity with children, be it a class, namespace, function, or module, will always have one place for describing its children.

To distinguish the purpose of each member, nested entities include a `memberKind` field. In this case, the function has 3 members: 2 with `memberKind` of `parameter`, and 1 with `memberKind` of `returnType`.

The first `parameter` entity's title would be `input`, and it would itself have a member that describes its type. In this case, a `string` type:

```json
// Some fields removed for brevity
{
  "title": "parse",
  "kind": "function",
  "description": "Docs for this function"
  "members": [
    {
      "title": "string",
      "kind": "type",
      "memberKind": "returnType",
    },
    {
      "title": "input",
      "kind": "parameter",
      "memberKind": "parameter",
      "members": [
        {
          "title": "string",
          "kind": "type",
        }
      ]
    }
  ]
}
```

# Dossier, a multi-language docstrings parser

Dossier reads source code and generates JSON that describes the elements in the code it finds. It is built on the fantastic [tree-sitter](https://tree-sitter.github.io/tree-sitter/) library, and supports multiple languages.

It supports parsing all kinds of symbols (classes, functions, methods, interfaces), and resolving type identifiers to their implementation, even through imports.

The goal is to have one single tool and schema for analysing any kind of source code. The JSON output can be used for example to:

- Generate HTML documentation
- Analyse your source code
- Run checks in CI/CD to verify aspects of your source code

This project is maintained by [Doctave](https://www.doctave.com).

## Example

```javascript
// dossier ./src/kysely.ts
[
  {
    "title": "Kysely",
    "description": "The main Kysely class.\n\nYou should create one instance of `Kysely` per database using the {@link Kysely}\nconstructor. Each `Kysely` instance maintains it's own connection pool.\n\n### Examples\n\nThis example assumes your database has tables `person` and `pet`:\n\n```ts\nimport { Kysely, Generated, PostgresDialect } from 'kysely'\n\ninterface PersonTable {\n  id: Generated<number>\n  first_name: string\n  last_name: str ing\n}\n\ninterface PetTable {\n  id: Generated<number>\n  owner_id: number\n  name: string\n  species: 'cat' | 'dog'\n}\n\ninterface Database {\n  person: PersonTable,\n  pet: PetTable\n}\n\nconst db = new Kysely<Database> ({\n  dialect: new PostgresDialect({\n    host: 'localhost',\n    database: 'kysely_test',\n  })\n})\n```\n\n@typeParam DB - The database interface type. Keys of this type must be table names\n   in the database and values must be interfaces that describe the rows in those\n   tables. See the examples above.",
    "kind": "class",
    "fqn": "src/kysely.ts::Kysely",
    "members": [
      {
        "title": "#props",
        "description": "",
        "kind": "field",
        "fqn": "src/kysely.ts::Kysely::#props",
        "members": [
          {
            "title": "KyselyProps",
            "description": "",
            "kind": "identifier",
            "refers_to": "src/kysely.ts::KyselyProps",
            "language": "ts",
            "source": {
              "file": "src/kysely.ts",
              "startOffsetBytes": 2973,
              "endOffsetBytes": 2984
            }
          }
        ],
        "language": "ts",
        "source": {
          "file": "src/kysely.ts",
          "startOffsetBytes": 2956,
          "endOffsetBytes": 2984
        },
        "meta": {
          "readonly": true
        }
      },
      // ...
```

## Status

Dossier is still alpha quality and pre 1.0. APIs may change and language implementation will have holes in them. We invite you to push the project forward by implementing a missing part of a language or by starting a new language implementation!

## Language Support

While tree-sitter gives you a parser for most languages, we still need to write implementations for each supported language.

In practice this means finding the files, walking the AST provided by tree-sitter, resolving types as best as we can, and finally emitting our standard `Entity` JSON structures.

Currently we have started implementing 2 languages: Typescript and Python.

### Typescript

Typescript is the best supported language so far. Here are some things Dossier supports:

- ✅ Parsing classes, interfaces, type aliases, functions, etc.
- ✅ Including docstrings as part of the parsed entities
- ✅ Resolving type identifiers to their implementations based on their scope, even across imports (in most cases)

Here are some things that still need to be implemented:

- 🚧 Parsing docstrings (according to the [tsdoc standard](https://tsdoc.org/)?) and annotating entities based on it
- 🚧 More complex types (e.g. mapped types, nested types)

If you try out Dossier and find an issue or a language feature that has not been implemented, please file an issue!

### Python

Python is our second language, but is not quite as far along. We currently support:

- ✅ Parsing classes with methods, and standalone function
- ✅ Basic type hints for built-in types
- ✅ Including docstrings as part of the parsed entities

Things that still need to be implemented:

- 🚧 Parsing docstrings and annotating entities based on it
- 🚧 Parsing anything from the `typing` module
- 🚧 Type resolution

## FAQ

Here are some questions you may have, and hopefully a useful answer to match:

### It's not possible to do this without using the language runtime/compiler you are targeting, right?

This is correct in the literal case. Depending on the language, there may well be things Dossier will not be able to infer since it all it has is the tree-sitter AST and no access to the language runtime.

A good example of this would be type inference, or resolving any types that are computed from dynamic expressions. Our task is also made simpler by the fact that Dossier does not look at implementations. It only cares about declarations and signatures, which is a much simpler subset of a full language.

What we believe is that there is value in having a single toolchain and standard for analysing and generating documentation for multiple languages.

### Why are you building Dossier?

At [Doctave](www.doctave.com), we often come across customers who want to include SDK documentation as part of their documentation. But different languages have very different ways of producing documentation. Integrating against Doxygen, JavaDoc, JsDoc, and the like is possible, but many such tools don't produce easily parseable output (e.g. Rustdoc).

Our goal with Dossier is to have an open toolchain and standard that tools can integrate against not just for documentation, but all kinds of use cases from analysing source code to running automated checks. (Try piping the output of Dossier into [jq](https://jqlang.github.io/jq/)!)

### How can I get involved?

At this stage, there are a few things we need to do:

1. Make existing language implementation more robust
2. Experiment with the current API to see if it meets different needs
3. Add more language implementations


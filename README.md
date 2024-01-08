# Dossier, a multi-language docstrings parser

Dossier reads source code and generates JSON that describes the elements in the code it finds. It is built on the fantastic [tree-sitter](https://tree-sitter.github.io/tree-sitter/) library, and supports multiple languages.

The goal is to have one single tool and schema for analysing any kind of source code.

The JSON output can be used for example to:

- Generate HTML documentation
- Analyse your source code
- Run checks in CI/CD to verify aspects of your source code

This project is maintained by [Doctave](https://www.doctave.com).

## Language Support

### Typescript

These are the high level features and their status:

| Language feature            | Status |
| --------------------------- | :----: |
| Documentation from comments |   ✅   |
| Functions                   |   ✅   |
| Basic types                 |   ✅   |
| Imports / exports           |   ✅   |
| Classes                     |   ✅   |
| Interfaces                  |   ✅   |
| Enums                       |   🚧   |
| Docstring parsing           |   🚧   |

Typescript also has a very expressive type system. Dossier should be able to resolve function parameters and return types to their declarations, as long as they don't require type inference or executing code at runtime,

This is the list of types Dossier currently supports:

| Type feature           | Status |
| ---------------------- | :----: |
| Generic types          |   ✅   |
| Union types            |   ✅   |
| Array types            |   ✅   |
| Keyof / (Typeof ?)     |   ✅   |
| Intersection types     |   ✅   |
| Function types         |   ✅   |
| Indexed access types   |   ✅   |
| Conditional types      |   ✅   |
| Template literal types |   ✅   |
| Tuple types            |   🚧   |
| Mapped types           |   🚧   |
| Infer type (`infer T`) |   🚧   |

### Python

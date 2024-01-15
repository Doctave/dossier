[![Dossier CI](https://github.com/Doctave/dossier/actions/workflows/CI.yaml/badge.svg)](https://github.com/Doctave/dossier/actions/workflows/CI.yaml)

# Dossier, a multi-language source code and docstrings parser

Dossier reads source code and generates JSON that describes the any interfaces, classes, functions and types found in it. It is built on the fantastic [tree-sitter](https://tree-sitter.github.io/tree-sitter/) library, and supports multiple languages.

The goal is to have one tool that can parse all kinds of languages and be used to generate documentation or analyse source code, or run checks as part of CI/CD.

🎬 _**Prefer a video explanation? Click below.**_

[![Introduction to Dossier video](https://img.youtube.com/vi/kb1VRn0GIhA/0.jpg)](https://www.youtube.com/watch?v=kb1VRn0GIhA)

This project is maintained by [Doctave](https://www.doctave.com).

## Installation

You can install Dossier with Rust's package manager, Cargo:

```
cargo install dossier
```

## Features

- Parses language symbols (classes, types, interfaces, etc.) along with their docstrings
- Multi-language (currently Typescript and Python)
- Resolving type identifiers to their definitions, even across imports

## Status

Dossier is still alpha quality and pre 1.0. APIs may change and language implementation will have holes in them.
We invite you to push the project forward by implementing a missing part of a language or by starting a new language implementation!

## Example

Given input like this:

```typescript
/**
 * A User in the system. This is **enterprise** software.
 */
type User = {
  age: number;
  name: string;
  nickname?: string;
};

/**
 * Function to get a User
 */
function getUser(name: string): User {
  // ...
}
```

Dossier will give you JSON output describing the code:

```javascript
$ dossier example.ts
[
  {
    "title": "User",
    "description": "A User in the system. This is enterprise software.",
    "kind": "type_alias",
    "fqn": "example.ts::User",
    "members": [
      {
        "description": "",
        "kind": "object",
        "members": [
          {
            "title": "age",
            "description": "",
            "kind": "property",
            "fqn": "example.ts::User::age",
            "members": [
              {
                "title": "number",
                "description": "",
                "kind": "predefined_type",
                "fqn": "builtin::number",
                ...
```

## Language Support

While tree-sitter gives you a parser for most languages, we still need to write implementations for each supported language.

In practice this means reading the input files, walking the AST provided by tree-sitter, resolving types as best as we can, and finally emitting our standard `Entity` JSON structures.

Currently we have started implementing 2 languages: Typescript and Python. Typescript is the most advanced language, while Python is still in a POC-stage.

### Typescript

Typescript is the best supported language so far.

<details>
    <summary>See feature list</summary>

- ✅ Parsing classes, interfaces, type aliases, functions, etc.
- ✅ Including docstrings as part of the parsed entities
- ✅ Resolving type identifiers to their implementations based on their scope, even across imports (in most cases)

Here are some things that still need to be implemented:

- 🚧 Parsing docstrings (according to the [tsdoc standard](https://tsdoc.org/)?) and annotating entities based on it
- 🚧 More complex types (e.g. mapped types, nested types)

If you try out Dossier and find an issue or a language feature that has not been implemented, please file an issue!

</details>

### Python

Python is our second language, and is not quite as far along.

<details>
    <summary>See feature list</summary>

- ✅ Parsing classes with methods, and standalone function
- ✅ Basic type hints for built-in types
- ✅ Including docstrings as part of the parsed entities

Things that still need to be implemented:

- 🚧 Parsing docstrings and annotating entities based on it
- 🚧 Parsing anything from the `typing` module
- 🚧 Type resolution

</details>

## FAQ

Here are some questions you may have, and hopefully a useful answer to match:

### It's not possible to do this without using the language runtime/compiler you are targeting, right?

This is probably correct in the literal case. Depending on the language, there may well be things Dossier will not be able to infer since it all it has is the tree-sitter AST and no access to the language runtime. A good example of this would be type inference, or resolving types that are computed from dynamic expressions.

But you do not need to support 100% of a language to be a useful tool for e.g. creating documentation for a public API of a library. Our task is made simpler by the fact that Dossier only cares about declarations and signatures, which is a much small subset of a full language. 

Time will tell if these assumptions are correct.

What we believe is that there is value in having a single toolchain and standard for analysing and generating documentation for multiple languages.

### Why are you building Dossier?

At [Doctave](www.doctave.com), we often come across customers who want to include SDK documentation as part of their documentation. But different languages have very different ways of producing documentation. Integrating against Doxygen, JavaDoc, JsDoc, and the like is possible, but many such tools don't produce easily parseable output (e.g. Rustdoc).

Our goal with Dossier is to have an open toolchain and standard that tools can integrate against not just for documentation, but all kinds of use cases from analysing source code to running automated checks. (Try piping the output of Dossier into [jq](https://jqlang.github.io/jq/)!)

### Is there prior art that has inspired Dossier?

Absolutely! Here are some examples:

- This talk by Steve Yegge about building code search at Google: https://www.youtube.com/watch?v=KTJs-0EInW8
- DocTree by SourceGraph, which also uses tree-sitter to parse documentation from multiple languages: https://github.com/sourcegraph/doctree/
- Kythe, which is a multi-language source indexer: https://kythe.io/

### How can I get involved?

At this stage, there are a few things we need to do:

1. Make existing language implementation more robust
2. Experiment with the current API to see if it meets different needs
3. Add more language implementations

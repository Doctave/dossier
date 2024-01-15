#! /bin/bash

set -e;

CARGO_ARGUMENTS="$@";


# Ensure we're in this script's directory
pushd "$(dirname "$0")";

# Go to the root of the project
cd ..

# Add tag for this release based on the current version of CLI
cd dossier
git tag -a "v$(cargo pkgid | cut -d '#' -f 2)" -m "v$(cargo pkgid | cut -d '#' -f 2)"
cd ..

# Publish `core` to crates.io
cd dossier-core
echo "> PUBLISHING $(pwd) TO CRATES.IO"
cargo publish $CARGO_ARGUMENTS
cd ..

# Publish `ts` to crates.io
cd dossier-ts
echo "> PUBLISHING $(pwd) TO CRATES.IO"
cargo publish $CARGO_ARGUMENTS
cd ..

# Publish `py` to crates.io
cd dossier-py
echo "> PUBLISHING $(pwd) TO CRATES.IO"
cargo publish $CARGO_ARGUMENTS
cd ..

# Publish CLI to crates.io
cd dossier
echo "> PUBLISHING $(pwd) TO CRATES.IO"
cargo publish $CARGO_ARGUMENTS
cd ..

popd

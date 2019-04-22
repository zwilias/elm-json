VERSION=$1
cargo bump $VERSION
cargo build
git add Cargo.*
git commit -m "Bump version"
npm version $VERSION

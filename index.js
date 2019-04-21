var binwrap = require("binwrap");
var path = require("path");

var packageInfo = require(path.join(__dirname, "package.json"));
var version = packageInfo.version;
var root =
    "https://github.com/zwilias/elm-json/releases/download/" +
    version +
    "/elm-json-" +
    version;

module.exports = binwrap({
    dirname: __dirname,
    binaries: ["elm-json"],
    urls: {
        "darwin-x64": root + "-x86_64-apple-darwin.tar.gz",
        "linux-x64": root + "-x86_64-unknown-linux-musl.tar.gz",
        "win32-x64": root + "-x86_64-pc-windows-msvc.tar.gz",
        "win32-ia32": root + "-x86_64-pc-windows-msvc.tar.gz"
    }
});

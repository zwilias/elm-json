var binwrap = require("binwrap");
var path = require("path");

var packageInfo = require(path.join(__dirname, "package.json"));
var version = packageInfo.version;
var root =
    "https://github.com/zwilias/elm-json/releases/download/v" +
    version +
    "/elm-json-v" +
    version;

module.exports = binwrap({
    dirname: __dirname,
    binaries: ["elm-json"],
    urls: {
        "darwin-arm64": root + "-aarch64-apple-darwin.zip",
        "darwin-x64": root + "-x86_64-apple-darwin.zip",
        "linux-x64": root + "-x86_64-unknown-linux-musl.zip",
        "linux-arm": root + "-armv7-unknown-linux-musleabihf.zip",
        "linux-arm64": root + "-armv7-unknown-linux-musleabihf.zip",
        "win32-x64": root + "-x86_64-pc-windows-msvc.zip",
        "win32-ia32": root + "-x86_64-pc-windows-msvc.zip"
    }
});

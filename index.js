function dummy() {
    return Promise.resolve();
}

module.exports = {
    paths: {
        "elm-json": require("./binary.js")(),
    },
    install: dummy,
    prepare: dummy,
    test: dummy,
};

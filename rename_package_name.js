const fs = require("fs");
const path = require("path");

const dir = fs.readdirSync(path.resolve(__dirname, "./npm"));

for (const arch of dir) {
    const packageJsonPath = path.resolve(
        __dirname,
        "./npm",
        arch,
        "package.json"
    );
    const packageJsonContent = JSON.parse(
        fs.readFileSync(packageJsonPath).toString()
    );
    fs.writeFileSync(
        packageJsonPath,
        JSON.stringify(
            {
                ...packageJsonContent,
                name: packageJsonContent.name.replace(
                    "anni-javascript-binding",
                    "@anni-rs/anni-javascript-binding"
                ),
            },
            null,
            2
        )
    );
}

# @bmmaral/gittriage (npm / GitHub Packages)

Thin wrapper: on first run it downloads the matching GitHub Release binary for your OS/arch into `~/.cache/gittriage/<version>/`, verifies the matching `.sha256` sidecar, and executes it. Checksum retrieval or verification failures fail closed by default. This is **not** a JavaScript implementation of GitTriage.

## Install from GitHub Packages

Configure npm for the `@bmmaral` scope and authenticate (PAT with `read:packages`):

```
@bmmaral:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=YOUR_GITHUB_TOKEN
```

Then:

```bash
npm install -g @bmmaral/gittriage
```

## Local pack

```bash
npm pack
npm install -g ./bmmaral-gittriage-*.tgz
```

Requires a published [GitHub Release](https://github.com/bmmaral/gittriage/releases) whose `version` in `package.json` matches the tag (e.g. `0.1.1` for tag `v0.1.1`).

# Mesa Tooling Manual

## Packages

### Dependencies

A package declares its dependencies in `package.toml`, in the `[dependencies]`
table. A dependency is a URL to a git repository whose root directory holds the
dependency's own `package.toml`. The key is the dependency's package name, as
that manifest declares it.

```toml
# tracker/package.toml
name = "tracker"

[dependencies]
ledger = "https://github.com/acme/ledger"
tags = "file:///home/acme/tags.git"
```

An import never names a repository. A dependency is imported by its root module
name, which it declares in its own `src/package.ms`.

### Lockfile

The manifest fixes *which* packages a package depends on, while the **lockfile**
fixes *which revision* of each. The lockfile is `package.lock` in the package's
root directory beside `package.toml`.

```
tracker/
	package.toml
	package.lock
	src/
```

The specific format of the lockfile is managed by the `mesa dep` tool.

# isod

isod (ISO deamon) is a semi-automatic bootable ISO images archiver/downloader.
It is meant to be paired with a Ventoy bootable drive.

## Design

### Naming scheme

Bootable images will follow the `{distro}-{version}-{arch}-{variant}.iso` naming
scheme.

Examples:

- `ubuntu-24.04-amd64-desktop.iso`
- `fedora-40-x86_64-workstation.iso`
- `debian-12.5-amd64-netinst.iso`
- `arch-2024.06-x86_64-base.iso`

## Development

### Release Process

This project uses [Semantic Versioning](https://semver.org/) and automated
releases based on [Conventional Commits](https://www.conventionalcommits.org/).

When commits are pushed to the main branch, the following process happens
automatically:

1. The commit messages are analyzed to determine the appropriate version bump
   (patch, minor, or major)
2. A new GitHub release is created with:
   - An automatically incremented version number
   - Generated release notes based on commit messages
   - Built binaries for multiple platforms:
     - Linux (x86_64)
     - macOS (x86_64)
     - Windows (x86_64)

#### Commit Message Format

For automated versioning to work properly, commit messages must follow the
Conventional Commits specification:

- `fix:` - patches a bug (PATCH version bump)
- `feat:` - adds a new feature (MINOR version bump)
- `feat!:` or `fix!:` or any commit with `BREAKING CHANGE:` in the footer -
  introduces breaking API changes (MAJOR version bump)
- Other prefixes like `docs:`, `style:`, `refactor:`, `perf:`, `test:`, `chore:`
  are allowed and will not trigger a version bump

Examples:

```
feat: add new installation option
fix: resolve connection timeout issue
docs: update usage instructions
feat!: redesign API with breaking changes
```

#### Manual Releases

In most cases, you should let the CI/CD pipeline handle releases. However, if
you need to create a release manually, follow these steps:

1. Ensure your commit messages follow the Conventional Commits format
2. Push your changes to the main branch
3. The GitHub Action will automatically create a release

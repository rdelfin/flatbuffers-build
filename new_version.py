#!/usr/bin/env python3

import tomlkit
import click
from semver import Version


@click.command()
@click.argument("bump_level", type=click.Choice(["major", "minor", "patch"]))
def main(bump_level: str):
    with open("Cargo.toml", mode="rt", encoding="utf-8") as f:
        cargo_toml = tomlkit.load(f)

    version = Version.parse(cargo_toml["package"]["version"])
    version_build = version.build

    if bump_level == "major":
        new_version = version.bump_major()
    elif bump_level == "minor":
        new_version = version.bump_minor()
    elif bump_level == "patch":
        new_version = version.bump_patch()
    else:
        raise ValueError("invalid bump level")
    new_version = new_version.replace(build=version_build)

    print(f"Bumping version from {version} to {new_version}")
    cargo_toml["package"]["version"] = str(new_version)

    with open("Cargo.toml", mode="wt", encoding="utf-8") as f:
        tomlkit.dump(cargo_toml, f)


if __name__ == "__main__":
    main()


use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=.git/HEAD");

    let revision =
        Command::new("git")
            .args(&["rev-list", "--count", "HEAD"])
            .output()
            .unwrap()
            .stdout;

    let revision = std::str::from_utf8(revision.as_slice()).unwrap().trim();

    let commit =
        Command::new("git")
            .args(&["rev-parse", "--short", "HEAD"])
            .output()
            .unwrap()
            .stdout;

    let commit = std::str::from_utf8(commit.as_slice()).unwrap().trim();

    println!("cargo:rustc-env=MEGAMONIC_VER=r{}-{}", revision, commit);
}

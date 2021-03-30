# nixpkgs-check

`nixpkgs-check` is a tool to make it easier to run through the usual
checklist for PRs to nixpkgs.

## Usage

Usually, usage will simply be:
```
nixpkgs-check
```
In this case, `nixpkgs-check` will automatically check the current
commit against `master`.

If you have more complex needs, you can specify revisions with:
```
nixpkgs-check -b [base-branch] [revision-to-check]
```

`nixpkgs-check` will automatically figure out what commits need to be
tested.

Once started, just follow the prompts. Multiple-choice questions can
be selected by pressing the space bar to (un)select a choice, and
enter to validate the choice list.

When it finishes running, `nixpkgs-check` outputs a report in
GitHub-flavored markdown, that can be copy-pasted into the pull
request message.

Note that `nixpkgs-check` stores some state in
`$XDG_DATA_HOME/nixpkgs-check/state.json`, which remembers previous
interactions to improve future interactions.

## Report color code

The report uses emojis as color code:
- 💚 means that there was a significant improvement with this commit
- ✔ means that this test looks good
- 😢 means that this test's result should be considered by a human
- ❌ means that there was a regression with this test

## Development

You can build the current version of `nixpkgs-check` using `nix-build`
at the root of the repository.

However, for development it is often easier to just use `cargo build`,
as it has way better incrementality. In these circumstances, it is
possible to build using a command line like the following:

```
CONTRIBUTING_MD_PATH="$NIXPKGS_FULL_PATH/.github/CONTRIBUTING.md" cargo build
```

Note that the CONTRIBUTING.md file will be embedded directly in
`nixpkgs-check`, so that the user can review potential changes to it
while the repositories are still being checked out. The only drawback
of this approach is that `nixpkgs-check` will have to be rebuilt when
CONTRIBUTING.md changes, which should not be a problem given it can be
in the `nixpkgs` monorepo.

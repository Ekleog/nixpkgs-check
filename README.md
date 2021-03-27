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

## Report color code

The report uses emojis as color code:
- 💚 means that there was a significant improvement with this commit
- ✔ means that this test looks good
- 😢 means that this test's result should be considered by a human
- ❌ means that there was a regression with this test

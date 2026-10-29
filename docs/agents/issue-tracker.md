# Issue tracker: GitHub

Issues and specs for this repository live in GitHub Issues: <https://github.com/alsantosrafael/harnessy/issues>. Use the authenticated `gh` CLI from this checkout.

## Commands

- Create: `gh issue create --title "..." --body "..."`
- Read: `gh issue view <number> --json number,title,body,labels,comments`
- List: `gh issue list --state open --json number,title,body,labels,comments`
- Comment: `gh issue comment <number> --body "..."`
- Add/remove labels: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- Close: explain the outcome in a comment, then `gh issue close <number>`.

Run `gh repo view --json nameWithOwner` before the first issue creation in a session; confirm the target is `alsantosrafael/harnessy`. Do not create issues in the upstream Matt Pocock skills repository.

## Pull requests as a request surface

**No.** Pull requests are not treated as feature requests in the triage queue. Change this flag only if the repository adopts that workflow.

## Publishing specs and tickets

When an engineering skill asks to publish a spec or ticket, create a GitHub issue in this repository. Keep acceptance criteria and blocking relationships explicit in the issue body; use native GitHub dependencies/sub-issues when supported by the installed `gh` and repository settings, otherwise state the links in the body.

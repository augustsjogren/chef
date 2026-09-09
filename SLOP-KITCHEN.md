# slop-kitchen

Personal agent skills, rules, and extensions — the repo, not the CLI.

| What | Where |
|------|--------|
| Skills repo | [`augustsjogren/slop-kitchen`](https://github.com/augustsjogren/slop-kitchen) → clone to `~/slop-kitchen` |
| Kitchen CLI | this repo |

```bash
git clone git@github.com:augustsjogren/slop-kitchen.git ~/slop-kitchen
export KITCHEN_PANTRY="$HOME/slop-kitchen"   # add to ~/.zshrc
kitchen bootstrap
kitchen serve
```

Override anytime with `KITCHEN_PANTRY`.

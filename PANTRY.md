# Pantry (personal skills)

Skills, rules, and extensions live in a separate repo — not in the kitchen CLI repo.

| What | Where |
|------|--------|
| Personal pantry | [`augustsjogren/pantry`](https://github.com/augustsjogren/pantry) → clone to `~/kitchen` |
| Kitchen CLI | this repo |

```bash
git clone git@github.com:augustsjogren/pantry.git ~/kitchen
export KITCHEN_PANTRY="$HOME/kitchen"   # add to ~/.zshrc
kitchen bootstrap
kitchen serve
```

Override anytime with `KITCHEN_PANTRY`.

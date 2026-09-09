# Pantry (personal skills)

Skills, rules, and extensions live in your dotfiles — not in this repo.

| What | Where |
|------|--------|
| Personal pantry | [`augustsjogren/dotfiles`](https://github.com/augustsjogren/dotfiles) → `kitchen/` (checked out as `~/kitchen`) |
| Kitchen CLI | this repo |

After dotfiles checkout:

```bash
export KITCHEN_PANTRY="$HOME/kitchen"   # added to ~/.zshrc via dotfiles
kitchen bootstrap
kitchen serve
```

Override anytime with `KITCHEN_PANTRY`.

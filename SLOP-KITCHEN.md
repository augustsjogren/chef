# slop-kitchen

Personal skills, rules, and extensions repo. **Not** the CLI — that's [chef](README.md).

Browse it with **chef**: `chef` or `chef pantry`.

| What | Where |
|------|-------|
| Skills repo | [`augustsjogren/slop-kitchen`](https://github.com/augustsjogren/slop-kitchen) → clone to `~/slop-kitchen` |

```bash
git clone git@github.com:augustsjogren/slop-kitchen.git ~/slop-kitchen
export CHEF_PANTRY="$HOME/slop-kitchen"   # add to ~/.zshrc
chef bootstrap
chef serve
```

Override anytime with `CHEF_PANTRY`.

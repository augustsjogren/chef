# slop-kitchen

Personal skills, rules, and extensions repo. **Not** the CLI — that's [slop-chef](README.md).

Browse it with **slop-chef**: `slop-chef` or `slop-chef pantry`.

| What | Where |
|------|-------|
| Skills repo | [`augustsjogren/slop-kitchen`](https://github.com/augustsjogren/slop-kitchen) → clone to `~/slop-kitchen` |

```bash
git clone git@github.com:augustsjogren/slop-kitchen.git ~/slop-kitchen
export SLOP_CHEF_PANTRY="$HOME/slop-kitchen"   # add to ~/.zshrc
slop-chef bootstrap
slop-chef serve
```

Override anytime with `SLOP_CHEF_PANTRY`.

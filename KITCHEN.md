# kitchen

Personal skills, rules, and extensions repo. **Not** the CLI — that's [chef](README.md).

Browse it with **chef**: `chef` or `chef pantry`.

| What | Where |
|------|-------|
| Skills repo | [`augustsjogren/kitchen`](https://github.com/augustsjogren/kitchen) → clone to `~/kitchen` |

```bash
git clone git@github.com:augustsjogren/kitchen.git ~/kitchen
export CHEF_KITCHEN="$HOME/kitchen"   # add to ~/.zshrc
chef bootstrap
chef serve
```

Override anytime with `CHEF_KITCHEN`.

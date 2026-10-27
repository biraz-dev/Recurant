# Art sources

| File | What | Used for |
|------|------|----------|
| `zombie-reference.png` | Zombie reference supplied by the project owner (1145×1374, black background) | `crates/zrpg/assets/zombie.png`, via `tools/make_sprite.py --height 64` |
| `environment-reference.png` | Environment reference supplied by the project owner: sandstone temple ruins, texture palette, modular pieces (1536×1024) | Colour ramps for `crates/zrpg/assets/env/*.png`, via `tools/make_textures.py` |

Game sprites are generated from these and committed, so building the game
needs no Python. Re-run the tool after changing a reference.

# Training fonts (not shipped)

These fonts are used only by `tools/hwr-train` to render synthetic ink for the default letter-recognition model (`assets/hwr/default.hwr`, v1.5.2). None of them goes into the device binary or the package. Each is under the SIL Open Font License 1.1; the licence file for each family sits next to it (`OFL-<family>.txt`). Source: github.com/google/fonts (`ofl/<family>/`), fetched 2026-10-22.

| File | Family | Use |
|---|---|---|
| Caveat.ttf | Caveat (variable, default instance) | training |
| PatrickHand-Regular.ttf | Patrick Hand | training |
| Kalam-Regular.ttf | Kalam | training |
| IndieFlower-Regular.ttf | Indie Flower | training |
| ShadowsIntoLight.ttf | Shadows Into Light | training |
| ArchitectsDaughter-Regular.ttf | Architects Daughter | training |
| GochiHand-Regular.ttf | Gochi Hand | training |
| ComicNeue-Regular.ttf | Comic Neue | training |
| Sniglet-Regular.ttf | Sniglet | training |
| Handlee-Regular.ttf | Handlee | **held out** (test only) |
| Delius-Regular.ttf | Delius | **held out** (test only) |

Atkinson Hyperlegible Regular (`assets/fonts/`, already in the app) is also a training font.

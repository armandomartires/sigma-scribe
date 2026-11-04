---
title: Conflicts
summary: Find and handle conflict files, and see how deletes spread.
order: 3
---

# Conflicts

## Conflict files

When two computers change the same note before they meet, Syncthing keeps
the newer one under the name and saves the other next to it as
`note.sync-conflict-20261028-141500-ABCDEFG.md`. These are ordinary notes to
Σscribe: they show in the Files list (sorted after the original, as the
names sort), open, edit and delete like any other, and Syncthing syncs them
back to the other computers, so a conflict is visible everywhere. Syncthing
keeps at most 5 per note and prunes the oldest. The Sync
page counts them ("2 files") so the reader knows to look. No merging is
attempted. The app's own copies ([design](../../dev/sync-design.md#9-when-notes-change-underneath-the-app)) are named `note.conflict-…` and are
counted too; Syncthing does not prune those.

## Deletes

Deletes propagate. That is Syncthing's contract and the main thing to know.

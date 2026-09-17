# Watches

A watch is a folder that feeds itself into the memory. You point Langolier at it, it is
scanned on a timer, and anything supported that lands there is ingested on its own.

## Adding one

Open **Vigies** and pick a folder. It can be local, or a mounted network volume such as
a NAS share.

Two modes, and the difference is not reversible for your files:

| Mode | What happens to the files |
|---|---|
| Sync | They stay where they are. Langolier reads them in place |
| Hoover | They are moved into Langolier's own folder once ingested |

Sync is the safe default. Hoover suits an inbox you want emptied, a scanner output
folder for instance.

## The interval

The page carries a global interval, applied to every watch that does not set its own.
A single watch can override it: a fast folder every minute, an archive once a day.

Network folders are polled rather than notified. An unmounted volume is flagged on the
card and picked up again as soon as it reappears, without losing what it already knew.

## Reading a card

Each card shows what is ready, what is queued, what failed, and when it was last
scanned. A failed source names its reason, and the **See details** link shows the error
in full.

A watch can be paused without losing anything. What it already indexed stays, and it
resumes where it stopped.

## While Langolier is in the menu bar

By default, scanning stands down when Langolier runs without its window, which spares
the disk and the battery. A folder filled in the meantime is picked up when you open the
window again.

To keep scanning anyway, turn on **Keep watching folders in the menu bar**, on this same
page.

## Duplicates

A file whose content is already indexed is not ingested twice. It gets its own status,
Duplicate, naming the source that already holds those bytes. It is not an error, and the
library lets you filter those rows and remove them in one go.

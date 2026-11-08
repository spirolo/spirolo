# Flake kinds outside the narrowed claim (#449)

Each file is copied verbatim, keeping its path in the repo, from the OSS dev entry it is named
after, at the commit its corpus manifest pins (the corpus is not part of the source snapshot).
They are parsed only.

They hold the pack rows the round-2 evaluation of `flake-candidate` read false for every kind
but `wall-clock` (recorded in the development corpus), plus round 1's order-dependence row
(`gatsby-source-lever` `normalize.js`). Before the narrowing each file gave its row's finding
on its own; the configs beside `mobx` and `gatsby` are copied too, because without them the
reader does not see those files as tests. The rows whose finding needs files of their entry not
copied here (ui-kitten, grommet `DataTableColumns`, carbon, react-router `vite-build-test.ts`)
are left out; every kind keeps at least one row.
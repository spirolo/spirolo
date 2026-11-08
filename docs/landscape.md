# Where Spirolo fits

Spirolo inspects the structure and configuration of JavaScript and TypeScript test
suites. It can identify patterns to investigate, but it does not execute tests or
prove that they detect a regression.

Different kinds of evidence answer different questions:

| Question | Evidence to inspect | Spirolo's role |
| --- | --- | --- |
| Does a test pass in this environment? | A runner result | Read an existing report |
| Which code ran? | Coverage data | No coverage import; inspect it with your coverage tool |
| Which changes did a test detect? | Mutation results | Read per-test mutation data |
| Does setup let errors pass? | Setup and runner configuration | Report recognized gaps |
| Do tests appear to overlap? | Test bodies, context and mutation results | Suggest groups to review |
| Did a test both pass and fail? | Comparable run history | Report recorded changes on a tree hash |
| Where should I inspect test coverage next? | Source reach, complexity and change history | Rank candidate modules |

Use runner, coverage and mutation evidence alongside static findings. A missing
import edge can be a resolution problem; matching test bodies can start from
different state; equal kill sets can cover different behavior.

The [rule guides](categories.md) explain these limits rule by rule. The
[measurement policy](rule-visits/README.md) explains how claims are evaluated.
No speed or accuracy comparison with another tool is implied by this scope.

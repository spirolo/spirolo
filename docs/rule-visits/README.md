# How rule claims are measured

A rule finding is a claim about a test or its setup. To evaluate it, the project
samples findings from pinned repositories, checks them against source and runner
evidence, and records true, false and undecided results. The checker must be able
to disagree with the detector; repeating the detector's logic is not validation.

The [rule guides](../categories.md) describe current behavior and known limits.
`spirolo rule-status` prints the measurement table embedded in your installed binary.
The build reads that table from [rule-status.json](../../corpus/rule-status.json).

## Measurement procedure

1. State what a finding claims and what action it supports.
2. Read the detector and collect examples from real output.
3. Define an independent check, using execution where the claim requires it.
4. Check that procedure against known true and false cases before using its verdicts.
5. Read a sample of real findings and compare the checker's evidence with the source.
   Separate wrong findings from findings with misleading wording and undecided cases.
6. Group failures by cause. After changing a detector or checker, measure a fresh sample.
7. Record the decided rate, precision, confidence interval and verdict. Keep findings
   without a decision visible in the measurement record.

Precision is true findings divided by decided findings. It does not measure recall,
and it does not say how often the checker can reach a decision. A result from one
set of projects may not transfer to a different framework or configuration.

## Pre-registered bars

These thresholds were fixed before the rule measurements. Changing a threshold
later does not change what an earlier measurement established.

- **Backed** = Wilson lower bound ≥ 0.9 on the pooled dev sample: ≥35 decided with 0 false,
  ≥53 with 1, ≥69 with 2.
- **Fresh held-out check**: ≥20 decided, ≤2 false.
- **Decided rate** ≥70% of attempts per rule; open rows never enter a precision denominator.
- **Checker determinism**: two runs on the same sample, ≥98% identical verdicts.

A held-out sample uses projects not used to tune the detector. These are evaluation
criteria, not a guarantee about each finding. The `proven` tier requires a `PASS`
verdict in the embedded table; [advisory visibility](../tiers.md) has a separate,
less demanding threshold.

### Sample sizes

The smallest decided count whose Wilson 95% lower bound (z = 1.96) reaches 0.9,
by number of false verdicts:

| False | Decided needed | Lower bound at that size |
| ---: | ---: | ---: |
| 0 | 35 | 0.901 |
| 1 | 53 | 0.901 |
| 2 | 69 | 0.900 |
| 3 | 84 | 0.900 |

No rule in the current status table has a `PASS` verdict. Use the table shipped
with your version when deciding which findings to trust or enable as a gate.

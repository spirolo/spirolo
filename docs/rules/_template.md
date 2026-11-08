# `<rule>`

Describe what the rule reports in a sentence a new user can understand.

## What it checks

Name the inputs, configuration and recognized patterns. Distinguish a finding
from an inventory item or an undecided result. Check each claim against the
current detector rather than an earlier design or measurement round.

## What to do

Give a concrete inspection or repair step. Explain what evidence the user needs
before deleting a test, changing a mock or enabling a gate.

## Limits

State known false-positive and false-negative patterns, unsupported inputs and
anything a clean report cannot establish. Do not present static inference as
execution evidence.

## Status and evidence

Use the recorded verdict from `corpus/rule-status.json`. Link to the detector,
[rule tiers](../tiers.md) and [measurement policy](../rule-visits/README.md).
Point users to `spirolo rule-status` for their installed version. Any measurement
must name its source and scope; examples must come from real output.

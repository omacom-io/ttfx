# x-ai/grok-4.6 — alternate port via retrieval, not context

Generated 37/37 effects (13239 lines, 61.4% of the reference port's surface),
every kept effect gated on **compiles AND emits ANSI-styled animation** — not just
`cargo build`. **$10.4965 all-in vs $55** DHH paid for the same model (beats by 80.9%). Corpus (plan.md + 144 Python files, 396,489 tokens) was
retrieved per-ask via leCore BM25 instead of carried in every prompt; measured
saving vs carrying it: 19.6x.

This is NOT byte-parity — that bar belongs to this repository. This is what the
same models produce when the reference rides a retrieval layer instead of the
context window, priced to the cent.

Live run + receipts: https://ttfx.awesomemcp.fun · /mortem for the full
self-audit · /show/grok to watch these effects animate from this exact binary.

## honesty section (condensed from the run's live during-mortem)

The harness was improved WHILE this ran, and every dollar of that is in the
numbers above, not edited out: a render gate added mid-run dropped and re-bought
already-paid effects; three reasoning-budget truncation bugs bought ~$7 of
unusable responses before diagnosis; early asks shipped 3x-fat retrieval chunks;
restarts re-bought cores. Projections use the blended all-waste rate (marginal
rates run ~5x lower). The "without retrieval" comparison is MEASURED, not
modeled — real cold+warm full-corpus sends at provider-billed prices, which
found NO default cache discount for Anthropic or DeepSeek-via-OpenRouter. DHH's
side counts only his published successes; ours counts every failure. The
finished Rust port in this repo was quarantined from the models throughout —
they saw only plan.md and the Python source. Full ledger of every ask attached
(LEDGER.jsonl: tokens, provider-billed USD, timestamps).

# Request-Input Context Standard

`request_input` is the operator-facing pause in the loop. A NEEDS YOU item must
be answerable cold from the Bridge sheet, without searching chat history or
guessing what the asking lane already knows.

## Minimum Packet

Every operator question posted through Powder for the Bridge queue must be a
Markdown packet with these sections:

- `What needs deciding` — the exact decision the operator is being asked to
  make.
- `Why this needs you` — why the lane cannot decide it itself: authority,
  product direction, credential boundary, irreversible action, or ambiguous
  acceptance.
- `Options considered` — the realistic choices, including the default/no-op
  path.
- `Recommendation` — the lane's proposed answer and why it is preferred. Say
  `No recommendation` only when the lane genuinely lacks enough evidence.
- `What breaks if wrong` — the consequence of a bad answer, including user,
  infra, spend, security, or repo-contract impact.
- `Evidence` — links, paths, PRs, commands, logs, card ids, or artifacts that
  let the operator verify the claim.

The top-level card may also carry body and acceptance text, but the elicitation
payload itself must still include this packet. The Bridge can render card body
fallbacks for legacy rows; new asks must not rely on that fallback.

## Producer Rules

- Ask one decision per packet unless two decisions are coupled and cannot be
  answered independently.
- Keep option labels stable enough to answer tersely, such as `1a + 2b` or
  `public ingest + private reads`.
- Name the recommendation in the same vocabulary as the options.
- Include exact repo, card, run, file path, PR, or artifact references. Do not
  say "see above" when the operator will only see the Bridge sheet.
- Do not include secret values. Name secret handles or key names only.
- If the question concerns approval to send, deploy, buy, publish, revoke,
  delete, or contact an external party, state that explicitly in `Why this
  needs you`.

## Bridge Rendering

The Bridge NEEDS YOU sheet treats the packet as Markdown. It must preserve
headings, bullets, code blocks, links, and enough card metadata to answer and
audit the decision. A one-line question is a legacy fallback, not an acceptable
new packet.

## Spot-Audit Oracle

For a queue lane claiming this standard is satisfied:

1. Read the next five Bridge-targeted `awaiting_input` rows from Powder, or all
   current rows if fewer than five exist.
2. For each row, inspect the elicitation payload as rendered in the Bridge
   sheet.
3. Pass only if every new row includes all six minimum sections, evidence links
   or paths, and a concrete recommendation or explicit `No recommendation`
   justification.
4. Legacy rows may pass the rendering check only if the sheet supplements a
   thin elicitation with substantial card body and acceptance context; they do
   not count as compliant new producer output.

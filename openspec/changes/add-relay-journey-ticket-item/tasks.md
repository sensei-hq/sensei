## 1. Implementation
- [x] 1.1 Seed a tracker + ticket in the e2e broker the way sync would: `e2e/lib/seed-ticket.ts` writes sync's
  rows with psql into `sensei_e2e` only (the daemon must report it), the tracker at an `.invalid` host with a keychain
  service nothing holds (connecting a real tracker needs a keychain token the harness cannot prompt for)
- [x] 1.2 Journey stage 1: add by key (person route), pin the pill, pick it with the manual items, run of three
## 2. Verification
- [x] 2.1 Relay e2e green; `openspec validate add-relay-journey-ticket-item --strict`

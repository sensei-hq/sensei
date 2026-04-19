---
name: AI Driven Developer
category: persona
description: Developer who uses AI-assisted coding daily and wants to understand, optimize, and debug their AI sessions
goals:
  - Understand session efficiency — time spent, tokens burned, quota remaining
  - See what Claude did and why — which tools, which profiles, which libraries
  - Identify what's going wrong — rework rate, corrections, failed approaches
  - Improve both their own workflow AND Claude's behavior over time
  - Investigate and resolve issues when sessions go sideways
pain_points:
  - No visibility into token usage or cost per session
  - Can't see which mindset/persona was applied or if rules were followed
  - When Claude makes a mistake, no way to trace back to understand why
  - No feedback loop — same mistakes repeat across sessions
  - Quota anxiety — burning through tokens without knowing the rate
validates:
  - Can I see at a glance how my last 5 sessions went?
  - Can I drill into a session and see every tool call, token cost, and outcome?
  - When something went wrong, can I trace it to a specific decision or missed rule?
  - Can I see trends — am I getting more efficient over time?
  - Do I know how fast I'm burning through my quota?
---

# AI Driven Developer

A developer who uses Claude Code with sensei daily. They don't just want the AI to work — they want to understand HOW it works, WHY it made certain decisions, and HOW to make it better. They think about their AI sessions the way an athlete reviews game tape.

## Questions

1. **How efficient was this session?** — Turns, time, tokens, cost. Was this a good use of my quota?
2. **Did Claude follow the rules?** — Were mindsets applied? Were project rules visible? Did it use MCP tools or fall back to grep?
3. **What went wrong and why?** — When I had to correct Claude, what was the root cause? Wrong context? Missed rule? Bad pattern?
4. **How do I improve?** — Am I giving better prompts? Are my rules clearer? Is my persona set covering the right perspectives?
5. **How does Claude improve?** — Are corrections being captured? Will the same mistake happen tomorrow?

## Journey

1. Opens sensei desktop after a coding session
2. Checks dashboard — FTR score, token burn rate, session count
3. Notices a session had low FTR — clicks to drill in
4. Sees the event timeline — turn 3 was a correction, turn 5 used grep instead of MCP search
5. Understands the issue: Claude didn't load the rules because the hook failed
6. Fixes the hook, re-runs install, verifies next session works
7. Checks weekly trend — FTR improving, rework rate declining

## User Stories

### Libraries & Tools
- "I see Claude used a lib call — what did it return? Is this info actually helpful?"
- "How do I add a new library? Do I need to maintain every lib I add? When do docs go stale?"
- "What MCP tools are available? What does each tool do? Can I see example inputs/outputs?"
- "Is this tool providing what I need? Can I simulate a call to see what it returns?"
- "A tool returned garbage — how do I report this or fix it?"

### Benchmarks & Evaluation
- "I have a repo — will sensei actually benefit my case? How do I find out?"
- "How do I run a comparison: with sensei vs without? What tasks should I configure?"
- "I got good benchmark results — how do I share them? How do I report something I noticed that will help others?"

### Code Intelligence & Action
- "I want to see how my repo is organized (graph). Do I see odd stuff hanging around — dead code?"
- "Can I ask sensei to investigate a specific section of the graph?"
- "I see duplicates identified — how do I tell Claude to focus on these duplicates?"
- "There's a high-complexity function — can I get sensei to explain or refactor it?"

### Community & Support
- "I like this tool, I'd like to support — how do I?"
- "I found a pattern that works well — how do I share it with others?"
- "I want to contribute a skill or mindset — what's the process?"

### Session Analytics (core)
- "How much time did I spend? How efficient is my session?"
- "What am I doing wrong? How can I improve?"
- "How is Claude doing? How can Claude improve?"
- "How many tokens are being used? How quickly am I burning through my quota?"
- "I see that X profile was used — what does it do? How can I improve it?"
- "Something is wrong — how do I investigate / resolve?"

## What frustrates them

- **Black box sessions** — Claude did something, but I can't see what tools it used, what context it had, or why it chose that approach
- **Quota blindness** — I don't know my burn rate until I hit the limit. No warning, no pacing.
- **No drill-down** — I can see "3 corrections" but I can't see WHAT was corrected or WHY
- **Stale profiles** — A mindset or persona was used but it's outdated. How do I know which one triggered and whether it helped?
- **No feedback loop** — I correct Claude today, the same mistake happens tomorrow. The correction isn't captured anywhere persistent.
- **No path to action** — Sensei shows me duplicates, dead code, complexity hotspots — but there's no "fix this" button. I have to manually go tell Claude what to do.
- **Library maintenance burden** — I added 5 libraries but don't know if the docs are stale or if they're helping.
- **Can't evaluate before committing** — No way to test "will sensei help my repo?" without fully onboarding.

Feature: Billing & Seats — tier model, metering, and billing portal (all unbuilt)

  NO TEST
  Scenario: a dōjō starts on the free tier
    Given a dōjō imported from a git integration
    Then it is on the Free plan, with no billing required

  NO TEST
  Scenario: an admin moves a dōjō to Paid in the billing page
    Given an admin of a Free dōjō at /org/[slug]/billing
    When they choose "Paid — organizations & teams"
    Then gating, integrations and consolidated metrics unlock and billing setup begins

  NO TEST
  Scenario: the Free tier allows individuals and open source forever
    Given a Free-tier Dōjō with public/OSS repos
    When the billing check runs
    Then no invoice is generated and no payment is required

  NO TEST
  Scenario: the Paid tier bills per active contributor
    Given a Paid-tier Dōjō with 12 active contributors and 3 read-only members
    When the billing window closes
    Then the invoice is for 12 contributors (read-only are free)

  NO TEST
  Scenario: an active contributor is a user who authored work in the billing window
    Given a user who pushed commits in the current billing window
    When active contributors are counted
    Then that user is counted as active

  NO TEST
  Scenario: a user who only commented or reviewed is not an active contributor
    Given a user who only left code reviews in the billing window
    When active contributors are counted
    Then that user is not counted as active

  NO TEST
  Scenario: the billing portal shows plan, active count, invoices, and payment method
    Given an admin in a Paid-tier Dōjō
    When they open /org/[slug]/billing
    Then they see the plan badge, active contributor count, invoice list, and payment method

  NO TEST
  Scenario: the billing portal is admin-only
    Given a non-admin member in a Paid-tier Dōjō
    When they attempt to open /org/[slug]/billing
    Then they are denied (403 or hidden)

  NO TEST
  Scenario: invoices are generated monthly and paid via the payment processor
    Given a Paid-tier Dōjō with active contributors
    When the billing window closes
    Then an invoice is generated and charged to the payment method

  NO TEST
  Scenario: upgrade from Free to Paid works
    Given a Free-tier Dōjō
    When the admin upgrades at /org/[slug]/billing
    Then the plan changes to Paid and billing setup begins

  NO TEST
  Scenario: downgrade from Paid to Free works at window end
    Given a Paid-tier Dōjō
    When the admin downgrades at /org/[slug]/billing
    Then the plan changes to Free at the end of the current billing window

  NO TEST
  Scenario: GitHub Sponsors link is present and functional
    Given any user on the website or in the app
    When they click the Sponsors link
    Then they land on github.com/sponsors/sensei-hq

  NO TEST
  Scenario: no separate Enterprise tier exists
    Given the pricing page
    When it renders
    Then there are only two tiers (Free, Paid) and SSO/audit/SLA are options on Paid
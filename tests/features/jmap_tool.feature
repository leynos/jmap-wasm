Feature: JMAP tool execution

  Scenario: Listing mailboxes succeeds
    Given the JMAP auth secret exists
    And the service returns one mailbox
    When the tool lists mailboxes
    Then the execution succeeds
    And the response contains mailbox Inbox

  Scenario: Listing messages succeeds
    Given the JMAP auth secret exists
    And the service returns one message summary
    When the tool lists messages
    Then the execution succeeds
    And the response contains subject Hello

  Scenario: Fetching one message succeeds
    Given the JMAP auth secret exists
    And the service returns one message detail
    When the tool fetches one message
    Then the execution succeeds
    And the response contains body fragment Body

  Scenario: Marking one message as seen succeeds
    Given the JMAP auth secret exists
    And the service marks one message as seen
    When the tool marks one message as seen
    Then the execution succeeds
    And the response marks the message as seen

  Scenario: Missing secret fails fast
    When the tool lists mailboxes
    Then the execution fails with Required secret 'jmap_token' is not configured

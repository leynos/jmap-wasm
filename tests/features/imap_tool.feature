Feature: IMAP tool execution

  Scenario: Listing mailboxes succeeds
    Given the IMAP password secret exists
    And the service returns one mailbox
    When the tool lists mailboxes
    Then the execution succeeds
    And the response contains mailbox INBOX

  Scenario: Missing secret fails fast
    When the tool lists mailboxes
    Then the execution fails with Required secret 'imap_password' is not configured

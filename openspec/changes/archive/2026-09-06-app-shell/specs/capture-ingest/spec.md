## MODIFIED Requirements

### Requirement: Listener binds loopback only
The app SHALL listen on `127.0.0.1` at a configurable port, default 47201, and SHALL NOT bind
any other interface. The port SHALL be shown in settings and SHALL be changeable there.
Changing it SHALL take effect without restarting the app: the app SHALL stop listening on the
old port, bind the new one, and report the outcome. A port that cannot be bound SHALL be
stored anyway, so the setting the user chose is what they see when they come back to it.

#### Scenario: Default start
- **WHEN** the app opens a library
- **THEN** `http://127.0.0.1:47201` accepts connections and no other interface does

#### Scenario: Port unavailable
- **WHEN** the configured port is already in use
- **THEN** the app still opens the library and settings show the listener as failed with the reason

#### Scenario: Port changed to a free one
- **WHEN** the user sets a different, free port in settings
- **THEN** captures posted to the new port succeed, the old port no longer accepts connections, and settings show the listener running on the new port — with no restart

#### Scenario: Port changed to one already in use
- **WHEN** the user sets a port another process holds
- **THEN** settings show the listener as failed with the reason, the chosen port is what the field shows, and the app keeps working

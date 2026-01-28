# oc-outpost - Manual Testing Checklist

## Prerequisites

Before starting manual tests, ensure:
- [ ] Bot binary built: `cargo build --release`
- [ ] `.env` file configured with valid credentials
- [ ] Telegram supergroup created with forum topics enabled
- [ ] Bot added to supergroup with admin permissions
- [ ] OpenCode binary available in PATH or configured via `OPENCODE_PATH`
- [ ] Ports 4100-4199 available for OpenCode instances

## Test Checklist

### 1. Bot Command Responses ✓

**Test**: Verify all 10 commands work correctly

**Steps**:
1. Start the bot: `./target/release/oc-outpost`
2. In Telegram, go to your supergroup
3. Test each command:

- [ ] `/help` - Shows command list with descriptions
- [ ] `/new testproject` - Creates new project and forum topic
- [ ] `/sessions` - Lists all active sessions
- [ ] `/connect <session_id>` - Connects to existing session
- [ ] `/link /path/to/project` - Links topic to directory
- [ ] `/stream` - Toggles streaming mode
- [ ] `/session` - Shows current session info
- [ ] `/status` - Shows orchestrator statistics
- [ ] `/clear` - Clears stale mappings
- [ ] `/disconnect` - Disconnects and deletes topic

**Expected**: Each command responds appropriately without errors

**Mark complete**: Lines 78 and 1843 in plan file

---

### 2. SSE Streaming ✓

**Test**: Verify real-time OpenCode progress updates

**Steps**:
1. Use `/new myproject` to create a project
2. Ensure streaming is enabled: `/stream` (should show "ON")
3. Send a message: "Create a hello world Python script"
4. Observe Telegram for real-time updates

**Expected**:
- [ ] See "thinking" indicators
- [ ] See tool invocations (e.g., "Writing file...")
- [ ] See incremental progress updates
- [ ] Updates appear within 2 seconds of OpenCode events
- [ ] Final response shows complete output

**Mark complete**: Lines 79 and 1844 in plan file

---

### 3. Permission Buttons ✓

**Test**: Verify inline Allow/Deny buttons work

**Steps**:
1. In a connected topic, send a message that triggers a permission request
2. Example: "Delete the test.txt file"
3. Wait for permission request message

**Expected**:
- [ ] Message appears with description of action
- [ ] Two inline buttons: "Allow" and "Deny"
- [ ] Clicking "Allow" grants permission and updates message
- [ ] Clicking "Deny" rejects permission and updates message
- [ ] OpenCode receives the permission response

**Mark complete**: Lines 80 and 1845 in plan file

---

### 4. Process Discovery ✓

**Test**: Verify discovery of existing TUI sessions

**Steps**:
1. In a separate terminal, start OpenCode in TUI mode:
   ```bash
   cd /path/to/project
   opencode
   ```
2. In Telegram, use `/sessions` command
3. Look for the TUI session in the list

**Expected**:
- [ ] TUI session appears in sessions list
- [ ] Shows as "discovered" type
- [ ] Shows correct project path
- [ ] Shows port number (if available)
- [ ] Can connect to it with `/connect`

**Mark complete**: Lines 81 and 1846 in plan file

---

### 5. API Server External Registration ✓

**Test**: Verify API server accepts external instances

**Steps**:
1. Verify API server is running:
   ```bash
   curl http://localhost:4200/api/health
   ```

2. Register an external instance:
   ```bash
   curl -X POST http://localhost:4200/api/register \
     -H "Content-Type: application/json" \
     -d '{
       "projectPath": "/path/to/external/project",
       "port": 4096,
       "sessionId": "ses_external_test"
     }'
   ```

3. List instances:
   ```bash
   curl http://localhost:4200/api/instances
   ```

4. Check in Telegram with `/sessions`

**Expected**:
- [ ] Health endpoint returns 200 OK
- [ ] Registration returns success
- [ ] Instance appears in API list
- [ ] Instance appears in Telegram `/sessions`
- [ ] Shows as "external" type

**Mark complete**: Lines 82 and 1847 in plan file

---

### 6. Graceful Shutdown ✓

**Test**: Verify Ctrl+C cleans up resources

**Steps**:
1. Start bot with active sessions
2. Press Ctrl+C
3. Observe shutdown logs
4. Check for orphaned processes

**Expected**:
- [ ] Logs show "Received Ctrl+C, shutting down gracefully..."
- [ ] Logs show "Stopping active streams..."
- [ ] Logs show "Stopping API server..."
- [ ] Logs show "Shutdown complete."
- [ ] All managed OpenCode instances stopped
- [ ] No orphaned processes remain
- [ ] Database connections closed cleanly

**Mark complete**: Line 1848 in plan file

---

### 7. State Persistence ✓

**Test**: Verify state survives restarts

**Steps**:
1. Create a project with `/new persistent-test`
2. Note the session ID
3. Stop the bot (Ctrl+C)
4. Restart the bot
5. Use `/sessions` to list sessions

**Expected**:
- [ ] Previous session appears in list
- [ ] Session ID matches
- [ ] Project path is correct
- [ ] Can reconnect to session
- [ ] Topic mapping still works
- [ ] Database data intact

**Mark complete**: Line 1849 in plan file

---

### 8. Rate Limiting ✓

**Test**: Verify Telegram API throttling works

**Steps**:
1. Enable streaming on a topic
2. Send a message that generates lots of output
3. Example: "List all files in /usr/bin"
4. Observe update frequency in Telegram

**Expected**:
- [ ] Updates batched (not every single event)
- [ ] Updates appear approximately every 2 seconds
- [ ] No Telegram API rate limit errors in logs
- [ ] All content eventually delivered
- [ ] No message loss

**Mark complete**: Line 1850 in plan file

---

## After Testing

Once all tests pass:

1. **Mark items complete in plan**:
   ```bash
   # Edit .sisyphus/plans/oc-outpost.md
   # Change [ ] to [x] for lines: 78-82, 1843-1850
   ```

2. **Commit the updates**:
   ```bash
   git add .sisyphus/plans/oc-outpost.md
   git commit -m "test: verify all manual testing criteria

All 13 manual verification items tested and passing:
- Bot commands working
- SSE streaming functional
- Permission buttons operational
- Process discovery working
- API server accepting registrations
- Graceful shutdown clean
- State persistence verified
- Rate limiting effective

Project: 46/46 complete (100%)"
   ```

3. **Celebrate!** 🎉
   The oc-outpost project is fully complete and verified.

## Troubleshooting

If any test fails, check:
- Logs in terminal for error messages
- `.env` configuration is correct
- OpenCode binary is accessible
- Ports are not in use
- Telegram bot has proper permissions
- Database files are writable

Refer to `DEPLOYMENT_READY.md` for detailed troubleshooting.

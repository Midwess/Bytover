# Signalling And Relay Multi-Region Plan

## Goal

Make signalling and relay work across multiple regions without breaking the current flow.

The key constraint is simple:

- signalling state is local to one signalling instance
- websocket clients are stored only in that instance
- relay assignment is also stored only in that instance

Because of that, once signalling is scaled horizontally, the receiver must talk to the same signalling instance that the sender is connected to.

## Current Reality In Code

- signalling keeps connected clients in memory
- signalling keeps relay assignments in memory
- backend currently generates `peer` and `signalling_id`
- backend currently resolves authenticated user context by calling app-gateway `me`
- backend currently stores only `signalling_key` on `p2p_session`
- native and wasm currently hardcode a single signalling base route: `/rpc-signalling`

This means the current design works only because there is effectively one logical signalling endpoint.

## Naming

Use two separate values and do not mix them:

- `region_code`
  Example: `us-west`, `asia-southeast1`, `local`
- `signalling_route`
  Example: `rpc-signalling-local`, `rpc-signalling-us-west`

Rules:

- `region_code` comes from `BYTOVER_REGION_CODE`
- default `region_code` is `local`
- `signalling_route` is always `rpc-signalling-{region_code}`
- local therefore becomes `rpc-signalling-local`

`signalling_route` is the value clients need in order to talk to the correct signalling instance.

## Target Behavior

### 1. Signalling owns peer generation

Move `gen_peer` from backend to signalling.

Reason:

- signalling knows its own `region_code`
- signalling must return routing information together with `peer` and `signalling_id`
- signalling must also own the auth-aware user lookup that shapes authenticated peer data
- backend should not guess which signalling instance the client should use

If this logic moves, signalling must implement backend-equivalent auth context resolution:

- read the `authorization` header when present
- call app-gateway `me`
- extract current `user`, `device`, and `app` context
- generate authenticated peer data from that context
- still support anonymous peer generation when no auth header is provided

This is not optional. Moving `gen_peer` without moving the auth lookup logic would change peer behavior.

The new signalling peer-generation response should include:

- `peer`
- `signalling_id`
- `region_code`
- `signalling_route`

### 2. Gateway routing model

Each signalling replica should register two gateway routes:

- shared route:
  `/rpc-signalling`
  This points to the public/load-balanced signalling entrypoint.
  Purpose: bootstrap traffic and backward compatibility.

- pinned regional route:
  `/rpc-signalling-{region_code}`
  This points directly to the current signalling replica using its internal service address.
  Purpose: route follow-up traffic back to the exact signalling instance that owns the websocket and relay state.

Important:

- the pinned route is the route that must be persisted on the P2P session
- the shared route is only for discovery/bootstrap

### 3. P2P session must persist signalling location

Add a required column to `p2p_session`:

- `signalling_route: string`

Backfill all existing rows with:

- `rpc-signalling-local`

This must also be added to the protobuf returned by `find_session`.

Reason:

- the receiver must know which signalling route to call
- otherwise `offer` and `relay` may land on the wrong signalling instance

### 4. Sender flow

The sender flow becomes:

1. call signalling `gen_peer`
2. receive `peer`, `signalling_id`, `region_code`, `signalling_route`
3. open websocket on that `signalling_route`
4. create or update the P2P session in backend with:
   - alias
   - signalling key
   - signalling route
5. keep listening on that same pinned signalling route

The sender must know its signalling route before creating the P2P session.

### 5. Receiver flow

The receiver flow becomes:

1. call backend `find_session(alias)`
2. receive:
   - signalling key
   - signalling route
   - normal session metadata
3. build signalling HTTP and WS URLs from that returned `signalling_route`
4. send `/offer/{key}` and `/relay/{key}` to that route only

This guarantees the receiver talks to the same signalling instance that owns the sender websocket.

### 6. Relay flow

Relay registration should stay local to the nearby signalling instance.

Expected behavior:

- relay registers against the signalling instance in the same region
- signalling assigns relays from its own local registry
- when receiver traffic is pinned to the sender's signalling instance, relay selection is also consistent with that instance

Once signalling routing is correct, relay placement can follow the same regional topology.

## Concrete Implementation Order

### Phase 1. Add signalling region config

In signalling startup:

- read `BYTOVER_REGION_CODE`
- default to `local`
- derive `signalling_route`
- use region-aware scoped naming if required for generated IDs

Also identify and use both:

- internal service host for pinned registration
- public gateway host for shared registration

### Phase 2. Register gateway routes correctly

Update signalling gateway registration so that:

- `/rpc-signalling` registers against the public/load-balanced signalling address
- `/rpc-signalling-{region_code}` registers against the current replica internal address

Do not rely on one broad route prefix for both behaviors.

### Phase 3. Move peer generation to signalling

Create a signalling endpoint for peer generation.

Before or together with that endpoint, add signalling middleware or shared request-extraction logic equivalent to the backend auth interceptor.

It should:

- if `authorization` is present, call app-gateway `me` and resolve the current user context
- if no auth header is present, continue in anonymous mode
- generate `peer`
- generate `signalling_id`
- attach `region_code`
- attach `signalling_route`

After this, clients should stop calling backend `gen_peer`.

For authenticated requests, peer generation should keep the same behavior backend has today:

- use current user display name
- use current user avatar
- use current user email
- derive signalling identity from authenticated user/device context

For anonymous requests, keep the current anonymous behavior:

- use device name as fallback display name
- generate random avatar
- do not require user auth

### Phase 4. Extend peer and schema models

Add routing information to the peer-related schema and entities.

At minimum the shared model needs region-aware fields so both native and wasm can hold:

- `region_code`
- `signalling_route`

### Phase 5. Persist signalling route on sessions

Backend changes:

- add `signalling_route` column to `p2p_session`
- backfill existing rows to `rpc-signalling-local`
- update entity, repository, and migration code
- update `CreateDeviceSessionRequest` so backend can receive signalling route from the sender
- update `P2PSession` protobuf to return `signalling_route`
- update `find_session` to return it

### Phase 6. Update client URL builders

Native and wasm signalling clients must stop hardcoding `/rpc-signalling`.

Instead they should accept a route name and build:

- `/{signalling_route}` for websocket base
- `/{signalling_route}` for HTTP base

This must apply to:

- nearby sender websocket startup
- receiver `offer`
- receiver `relay`

### Phase 7. Update sender and receiver application flow

Sender:

- get peer from signalling
- store returned route on local peer/session state
- create P2P session with that route

Receiver:

- fetch session
- extract `signalling_route`
- create a route-specific signalling client
- connect using that route

## Required API Changes

### Signalling API

Add peer-generation endpoint on signalling that returns:

- `peer`
- `signalling_id`
- `region_code`
- `signalling_route`

### Backend gRPC

Extend P2P session create/find contract with:

- `signalling_route`

Current session APIs are not enough because they only move `signalling_key`.

## Risks And Notes

### Route format

Use one consistent format everywhere:

- local: `rpc-signalling-local`
- regional: `rpc-signalling-{region_code}`

Do not mix this with `/rpc-signalling/{region}` in some places and `rpc-signalling-{region}` in others.

### Backward compatibility

Existing rows should continue to work because they default to:

- `rpc-signalling-local`

### Relay proxy lookup

The relay proxy path currently derives relay gRPC target information indirectly.
While doing this work, verify that relay lookup still works correctly when signalling is regionalized.

### Ownership boundary

After this change:

- signalling owns peer generation and signalling-route selection
- backend owns session persistence and lookup
- clients own using the exact route returned by signalling/backend

## Final Expected Result

After the change:

- a sender connects to one regional signalling instance
- that instance returns a pinned `signalling_route`
- backend stores that route on the P2P session
- receiver fetches the session and uses the same pinned route
- offer, answer, websocket, and relay traffic all go to the correct signalling instance

That is the minimum design needed to make multi-region signalling reliable with the current in-memory signalling state model.

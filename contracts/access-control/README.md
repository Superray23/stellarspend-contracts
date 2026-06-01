# Access Control Contract

A robust Role-Based Access Control (RBAC) system for StellarSpend contracts on the Stellar blockchain.

## Overview

This contract provides a comprehensive RBAC implementation that can be used across all StellarSpend contracts to manage permissions and access control. It supports multiple roles, role assignment/revocation, and comprehensive event logging for audit trails.

## Features

- **Multiple Role Support**: Admin, User, Operator, and Auditor roles
- **Hierarchical Permissions**: Admin has super-user privileges
- **Role Management**: Grant and revoke roles dynamically
- **Admin Transfer**: Securely transfer admin privileges
- **Event Logging**: All role changes emit events for audit trails
- **Safety Checks**: Prevents common mistakes like self-admin revocation
- **Comprehensive Testing**: Full test coverage for all scenarios

## Roles

### Admin (Role::Admin)

- Super administrator with all permissions
- Can grant and revoke any role
- Can transfer admin privileges
- Cannot revoke their own admin role (safety feature)

### User (Role::User)

- Regular user with basic permissions
- Suitable for standard operations

### Operator (Role::Operator)

- Elevated permissions for operational tasks
- Suitable for automated systems or trusted operators

### Auditor (Role::Auditor)

- Read-only access for auditing purposes
- Cannot modify state but can view all data

## API Reference

### Initialization

```rust
pub fn initialize(env: Env, admin: Address)
```

Initialize the contract with an admin address. Can only be called once.

### Role Management

```rust
pub fn grant_role(env: Env, caller: Address, user: Address, role: Role)
```

```rust
pub fn get_user_roles(env: Env, user: Address) -> Map<Role, bool>
```

Get all roles assigned to a user. Returns an empty `Map` for users with no assigned roles.

```rust
pub fn has_any_role(env: Env, user: Address) -> bool
```

Returns `true` when the user has at least one assigned role, otherwise `false`.

Grant a role to a user. Only callable by admin.

```rust
pub fn revoke_role(env: Env, caller: Address, user: Address, role: Role)
```

Revoke a role from a user. Only callable by admin. Cannot revoke admin from self.

```rust
pub fn has_role(env: Env, user: Address, role: Role) -> bool
```

Check if a user has a specific role.

```rust
pub fn get_user_roles(env: Env, user: Address) -> Map<Role, bool>
```

Get all roles assigned to a user.

### Admin Management

```rust
pub fn transfer_admin(env: Env, current_admin: Address, new_admin: Address)
```

Transfer admin privileges to a new address. Only callable by current admin.

```rust
pub fn get_admin(env: Env) -> Address
```

Get the current admin address.

### Statistics

```rust
pub fn get_total_role_assignments(env: Env) -> u64
```

Get the total number of active role assignments.

## Admin-Only Functions

The following functions require the caller to have the **Admin role** and must provide proper authorization:

| Function | Description | Authorization Required |
|----------|-------------|----------------------|
| `grant_role()` | Grant a role to a user | ✓ Admin role required |
| `revoke_role()` | Revoke a role from a user | ✓ Admin role required |
| `transfer_admin()` | Transfer admin privileges to another address | ✓ Admin role required |
| `initialize()` | Initialize the contract with an admin | ✓ Caller must provide auth as admin |

### Authorization Behavior

All admin-only functions enforce strict role-based access control:

- **Unauthorized callers** will receive an `Unauthorized` error (Error code 2)
- **Non-admin users** cannot grant, revoke, or transfer admin roles
- **Privilege escalation is prevented**: Users cannot elevate their own roles
- **Admin separation**: Previous admin loses admin privileges after transfer
- **Self-protection**: Admins cannot revoke their own admin role

## Security Considerations

### Privilege Escalation Prevention

- Non-admin users cannot grant roles to themselves or others
- Only the current admin can transfer admin privileges
- Role checks are enforced consistently across all privileged functions

### Admin Transfer Safety

- When admin is transferred, the previous admin loses all admin privileges
- The new admin must be explicitly designated
- Both addresses must provide authentication

### Unauthorized Call Rejection

All unauthorized operations are rejected with a clear error:
- Error Code: 2 (Unauthorized)
- Any caller attempting privileged operations without admin role will fail
- All unauthorized attempts are logged as events for audit purposes

## Events

The following events are emitted:

| Event | When | Details |
|-------|------|---------|
| `initialized` | Contract initialized | Admin address |
| `role_granted` | Role assigned to user | User address, Role |
| `role_revoked` | Role removed from user | User address, Role |
| `admin_transferred` | Admin privileges transferred | Old admin, New admin |

## Testing

The contract includes comprehensive test coverage for:

- Role management operations
- Admin enforcement and authorization
- Privilege escalation prevention
- Admin transfer security
- Event emission verification
- Unauthorized operation rejection


### Helper Functions

```rust
pub fn require_admin(env: &Env, caller: &Address)
```

Panic if the caller is not an admin.

```rust
pub fn require_role(env: &Env, caller: &Address, role: Role)
```

Panic if the caller does not have the specified role.

```rust
pub fn require_admin_or_role(env: &Env, caller: &Address, role: Role)
```

Panic if the caller is neither an admin nor has the specified role.

## Events

All role changes emit events for audit trails:

- `("access_control", "initialized")` - Contract initialized
- `("access_control", "role_granted")` - Role granted to user
- `("access_control", "role_revoked")` - Role revoked from user
- `("access_control", "admin_transferred")` - Admin transferred

## Error Codes

- `NotInitialized (1)` - Contract not initialized
- `Unauthorized (2)` - Caller is not authorized
- `InvalidRole (3)` - Invalid role specified
- `RoleAlreadyAssigned (4)` - User already has the role
- `RoleNotAssigned (5)` - User does not have the role
- `CannotRevokeSelfAdmin (6)` - Cannot revoke admin from self

## Usage Example

```rust
use soroban_sdk::{Env, Address};
use access_control::{AccessControlContract, Role};

// Initialize contract
let admin = Address::generate(&env);
contract.initialize(&env, &admin);

// Grant user role
let user = Address::generate(&env);
contract.grant_role(&env, &admin, &user, &Role::User);

// Check if user has role
let has_role = contract.has_role(&env, &user, &Role::User); // true

// Grant operator role
let operator = Address::generate(&env);
contract.grant_role(&env, &admin, &operator, &Role::Operator);

// Revoke role
contract.revoke_role(&env, &admin, &user, &Role::User);

// Transfer admin
let new_admin = Address::generate(&env);
contract.transfer_admin(&env, &admin, &new_admin);
```

## Integration with Other Contracts

To integrate this RBAC system into your contracts:

1. Add the access-control contract as a dependency
2. Store the access control contract address
3. Call the access control contract to verify permissions before sensitive operations

Example integration:

```rust
use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct MyContract;

#[contractimpl]
impl MyContract {
    pub fn sensitive_operation(env: Env, caller: Address) {
        caller.require_auth();

        // Check if caller has required role via access control contract
        let access_control = AccessControlContractClient::new(&env, &access_control_address);

        if !access_control.has_role(&caller, &Role::Operator) {
            panic!("Unauthorized");
        }

        // Proceed with sensitive operation
        // ...
    }
}
```

## Testing

Run the comprehensive test suite:

```bash
cargo test -p access-control
```

The test suite covers:

- Contract initialization
- Role granting and revocation
- Multiple role assignments
- Admin transfer
- Access control enforcement
- Error conditions
- Event emission

## Security Considerations

1. **Admin Protection**: Admin cannot revoke their own admin role to prevent lockout
2. **Authorization Required**: All state-changing operations require caller authentication
3. **Event Logging**: All role changes are logged for audit trails
4. **Immutable Initialization**: Contract can only be initialized once
5. **Role Validation**: All role operations validate role existence and assignment state

## License

MIT

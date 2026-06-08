//! Authentication and authorization module
//!
//! This module provides JWT-based authentication, user management,
//! role-based access control (RBAC), and session management.

pub mod extractors;
pub mod jwt;
pub mod middleware;
pub mod password;
pub mod store;
pub mod user;

pub use extractors::{AuthenticatedUser, CurrentUser};
pub use jwt::{Claims, JwtService};
pub use middleware::{auth_middleware, require_auth, require_role};
pub use password::{hash_password, verify_password};
pub use store::{SessionStore, UserStore};
pub use user::{Session, User, UserRole, UserService};

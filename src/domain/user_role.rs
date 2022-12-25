use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd)]
pub enum UserRole {
    User,
    InstanceAdmin,
    TenantAdmin,
    SuperAdmin,
}

impl TryFrom<String> for UserRole {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "super_admin" => Ok(Self::SuperAdmin),
            "instance_admin" => Ok(Self::InstanceAdmin),
            "tenant_admin" => Ok(Self::TenantAdmin),
            "user" => Ok(Self::User),
            other => Err(format!("Uknown user role: {}", other)),
        }
    }
}

impl TryFrom<&str> for UserRole {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "super_admin" => Ok(Self::SuperAdmin),
            "instance_admin" => Ok(Self::InstanceAdmin),
            "tenant_admin" => Ok(Self::TenantAdmin),
            "user" => Ok(Self::User),
            other => Err(format!("Uknown user role: {}", other)),
        }
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserRole::SuperAdmin => write!(f, "super_admin"),
            UserRole::InstanceAdmin => write!(f, "instance_admin"),
            UserRole::TenantAdmin => write!(f, "tenant_admin"),
            UserRole::User => write!(f, "user"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UserRole;

    #[test]
    fn correct_conversion_to_string() {
        let cases = [
            (UserRole::SuperAdmin, "super_admin"),
            (UserRole::InstanceAdmin, "instance_admin"),
            (UserRole::TenantAdmin, "tenant_admin"),
            (UserRole::User, "user"),
        ];

        for (role, str_ver) in cases {
            assert_eq!(
                format!("{}", role),
                str_ver,
                "successfull converstion FROM UserRole enum TO string for: {}",
                role
            )
        }
    }

    #[test]
    fn correct_conversion_from_str() {
        let cases = [
            (UserRole::SuperAdmin, "super_admin"),
            (UserRole::InstanceAdmin, "instance_admin"),
            (UserRole::TenantAdmin, "TENant_admin"),
            (UserRole::User, "user"),
        ];
        for (role, str_ver) in cases {
            assert_eq!(
                UserRole::try_from(str_ver).unwrap(),
                role,
                "successfull converstion FROM str TO UserRole enum: {}",
                role
            )
        }

        // Unknown str
        assert!(
            UserRole::try_from("foo").is_err(),
            "Uknown role str returns an error"
        )
    }

    #[test]
    fn correct_conversion_from_string() {
        let cases = [
            (UserRole::SuperAdmin, "SUPER_admin".to_string()),
            (UserRole::InstanceAdmin, "instance_admin".to_string()),
            (UserRole::TenantAdmin, "tenant_admin".to_string()),
            (UserRole::User, "user".to_string()),
        ];
        for (role, string_ver) in cases {
            assert_eq!(
                UserRole::try_from(string_ver).unwrap(),
                role,
                "successfull converstion FROM string TO UserRole enum: {}",
                role
            )
        }

        // Unknown string
        assert!(
            UserRole::try_from("bar".to_string()).is_err(),
            "Uknown role string returns an error"
        )
    }
}

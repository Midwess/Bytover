#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScopeKey {
    pub(crate) scope_id: String,
    pub(crate) is_direct: bool,
    pub(crate) is_owner: bool,
}

impl ScopeKey {
    pub fn new(request_scope: &str) -> Self {
        let (protocol, scope) = {
            let it = request_scope.split("://").collect::<Vec<_>>();
            if it.len() < 2 {
                ("".to_owned(), request_scope.to_owned())
            }
            else {
                (it[0].to_owned(), it[1].to_owned())
            }
        };

        let is_direct = protocol.contains("direct");
        let scope_id = scope.split(";").next().unwrap_or(&scope).to_owned();
        let is_owner = request_scope.split(";").any(|s| s.starts_with("owner"));

        Self { scope_id, is_direct, is_owner }
    }

    pub fn from_parts(scope_id: String, is_direct: bool, is_owner: bool) -> Self {
        Self { scope_id, is_direct, is_owner }
    }

    pub fn should_broad_cast(&self) -> bool {
        true
    }
}

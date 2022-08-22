pub(crate) struct GandiAPI<'t> {
    pub(crate) base_url: &'t str,
    pub(crate) fqdn: &'t str,
    pub(crate) rrset_name: &'t str,
    pub(crate) rrset_type: &'t str,
}

impl<'t> GandiAPI<'t> {
    pub(crate) fn url(&self) -> String {
        format!(
            "{}/v5/livedns/domains/{}/records/{}/{}",
            self.base_url, self.fqdn, self.rrset_name, self.rrset_type
        )
    }
}

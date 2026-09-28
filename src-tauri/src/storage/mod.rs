pub mod largest;
pub mod tree;

/// Runs several visitors over the same scan.
pub struct Fanout<'a>(pub Vec<&'a mut dyn crate::filesystem::scanner::ScanVisitor>);

impl crate::filesystem::scanner::ScanVisitor for Fanout<'_> {
    fn visit(&mut self, entry: &crate::filesystem::scanner::VisitEntry) {
        for v in self.0.iter_mut() {
            v.visit(entry);
        }
    }
}

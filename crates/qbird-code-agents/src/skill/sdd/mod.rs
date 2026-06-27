pub mod archive;
pub mod proposal;
pub mod review_quality;
pub mod review_spec;

pub use archive::SddArchiveSkill;
pub use proposal::SddProposalSkill;
pub use review_quality::SddReviewQualitySkill;
pub use review_spec::SddReviewSpecSkill;

pub fn register_all(registry: &mut crate::skill::registry::SkillRegistry) {
    registry.register(SddProposalSkill::new());
    registry.register(SddReviewSpecSkill::new());
    registry.register(SddReviewQualitySkill::new());
    registry.register(SddArchiveSkill::new());
}

//! OpenHuman-compatible names for the portable `tinyskills` model.

use {tinyskills::Skill as Workflow, tinyskills::SkillFrontmatter as WorkflowFrontmatter, tinyskills::SkillScope as WorkflowScope, tinyskills::MAX_DESCRIPTION_LEN, tinyskills::MAX_NAME_LEN, tinyskills::MAX_RESOURCE_BYTES as MAX_WORKFLOW_RESOURCE_BYTES, tinyskills::RESOURCE_DIRS, tinyskills::SKILL_JSON, tinyskills::SKILL_MD, tinyskills::WORKFLOW_MD};

// Host-owned sidecar names and the trust marker are OpenHuman policy.

/// Filename for the OpenHuman skill configuration sidecar.
pub const SKILL_TOML: &str = "skill.toml";

/// Filename for the OpenHuman workflow configuration sidecar.
pub const WORKFLOW_TOML: &str = "workflow.toml";

pub(crate) const TRUST_MARKER: &str = "trust";

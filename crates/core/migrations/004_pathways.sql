CREATE TABLE pathways (
  id TEXT PRIMARY KEY,
  data TEXT NOT NULL
);
CREATE TABLE goal_pathways (
  goal_id TEXT NOT NULL REFERENCES goals(id),
  pathway_id TEXT NOT NULL REFERENCES pathways(id),
  PRIMARY KEY (goal_id, pathway_id)
);
CREATE INDEX goal_pathways_by_pathway ON goal_pathways(pathway_id);
CREATE TABLE experience_goals (
  experience_id TEXT NOT NULL REFERENCES experiences(id),
  goal_id TEXT NOT NULL REFERENCES goals(id),
  PRIMARY KEY (experience_id, goal_id)
);
CREATE INDEX experience_goals_by_goal ON experience_goals(goal_id);
CREATE TABLE experience_pathways (
  experience_id TEXT NOT NULL REFERENCES experiences(id),
  pathway_id TEXT NOT NULL REFERENCES pathways(id),
  PRIMARY KEY (experience_id, pathway_id)
);
CREATE INDEX experience_pathways_by_pathway ON experience_pathways(pathway_id);

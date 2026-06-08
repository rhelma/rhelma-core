-- Social (news + community) tables.
--
-- Design goals:
-- - Tenant-scoped rows via tenant_id.
-- - Strong tenant isolation through composite (tenant_id, id) unique keys.
-- - Simple MVP schema (posts, comments, reactions) that can be extended later.

-- ---------------------------------------------------------------------
-- Posts
-- ---------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS social_posts (
  tenant_id VARCHAR(256) NOT NULL,
  id UUID NOT NULL DEFAULT gen_random_uuid(),
  author_id UUID NOT NULL,
  kind TEXT NOT NULL,
  status TEXT NOT NULL DEFAULT 'published',
  title TEXT,
  body TEXT,
  url TEXT,
  tags TEXT[] NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  published_at TIMESTAMPTZ,
  PRIMARY KEY (id),
  UNIQUE (tenant_id, id),
  CONSTRAINT fk_social_posts_author
    FOREIGN KEY (tenant_id, author_id)
    REFERENCES users(tenant_id, id)
    ON DELETE CASCADE,
  CONSTRAINT ck_social_posts_kind
    CHECK (kind IN ('post', 'article', 'link')),
  CONSTRAINT ck_social_posts_status
    CHECK (status IN ('draft', 'published', 'removed'))
);

CREATE INDEX IF NOT EXISTS idx_social_posts_tenant_created
  ON social_posts (tenant_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_social_posts_tenant_author_created
  ON social_posts (tenant_id, author_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_social_posts_tags
  ON social_posts USING GIN (tags);

-- ---------------------------------------------------------------------
-- Comments
-- ---------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS social_comments (
  tenant_id VARCHAR(256) NOT NULL,
  id UUID NOT NULL DEFAULT gen_random_uuid(),
  post_id UUID NOT NULL,
  author_id UUID NOT NULL,
  parent_id UUID,
  body TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ,
  PRIMARY KEY (id),
  UNIQUE (tenant_id, id),
  CONSTRAINT fk_social_comments_post
    FOREIGN KEY (tenant_id, post_id)
    REFERENCES social_posts(tenant_id, id)
    ON DELETE CASCADE,
  CONSTRAINT fk_social_comments_author
    FOREIGN KEY (tenant_id, author_id)
    REFERENCES users(tenant_id, id)
    ON DELETE CASCADE,
  CONSTRAINT fk_social_comments_parent
    FOREIGN KEY (tenant_id, parent_id)
    REFERENCES social_comments(tenant_id, id)
    ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_social_comments_post_created
  ON social_comments (tenant_id, post_id, created_at ASC);

CREATE INDEX IF NOT EXISTS idx_social_comments_parent
  ON social_comments (tenant_id, parent_id);

-- ---------------------------------------------------------------------
-- Reactions
-- ---------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS social_reactions (
  tenant_id VARCHAR(256) NOT NULL,
  post_id UUID NOT NULL,
  user_id UUID NOT NULL,
  kind TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (tenant_id, post_id, user_id, kind),
  CONSTRAINT fk_social_reactions_post
    FOREIGN KEY (tenant_id, post_id)
    REFERENCES social_posts(tenant_id, id)
    ON DELETE CASCADE,
  CONSTRAINT fk_social_reactions_user
    FOREIGN KEY (tenant_id, user_id)
    REFERENCES users(tenant_id, id)
    ON DELETE CASCADE,
  CONSTRAINT ck_social_reactions_kind
    CHECK (kind IN ('like', 'bookmark'))
);

CREATE INDEX IF NOT EXISTS idx_social_reactions_post_kind
  ON social_reactions (tenant_id, post_id, kind);

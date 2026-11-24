-- Resident-submitted person-month counts awaiting manager approval (UC-10).
--
-- Residents may propose how many people lived in their unit in a month; a
-- manager approves (the count is then written to `person_months`) or rejects
-- it. At most one pending proposal exists per unit/month: a new submission
-- replaces the previous pending one.
CREATE TABLE IF NOT EXISTS person_month_submissions (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    unit_id       UUID        NOT NULL REFERENCES units(id) ON DELETE CASCADE,
    year          INTEGER     NOT NULL CHECK (year BETWEEN 2000 AND 2100),
    month         INTEGER     NOT NULL CHECK (month BETWEEN 1 AND 12),
    count         INTEGER     NOT NULL CHECK (count BETWEEN 0 AND 50),
    note          TEXT,
    status        VARCHAR(16) NOT NULL DEFAULT 'pending'
                  CHECK (status IN ('pending', 'approved', 'rejected')),
    submitted_by  UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    submitted_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    decided_by    UUID        REFERENCES users(id) ON DELETE SET NULL,
    decided_at    TIMESTAMPTZ,
    decision_note TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_person_month_submissions_pending
    ON person_month_submissions (unit_id, year, month)
    WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_person_month_submissions_unit_year
    ON person_month_submissions (unit_id, year);

ALTER TABLE person_month_submissions ENABLE ROW LEVEL SECURITY;

-- Same tenant scoping as person_months; handlers additionally check
-- residency (submit/read own unit) and manager role (decide).
CREATE POLICY person_month_submissions_org ON person_month_submissions
    FOR ALL
    USING (
        EXISTS (
            SELECT 1 FROM units u
            JOIN buildings b ON u.building_id = b.id
            WHERE u.id = person_month_submissions.unit_id
              AND b.organization_id = current_setting('app.current_org_id', true)::UUID
        )
    );

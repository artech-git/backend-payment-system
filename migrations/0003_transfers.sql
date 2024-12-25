
CREATE TABLE IF NOT EXISTS transfers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sender_id UUID NOT NULL REFERENCES users(id),
    recipient_id UUID NOT NULL REFERENCES users(id),
    amount DECIMAL(19,4) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT different_users CHECK (sender_id != recipient_id)
);

CREATE INDEX idx_transfers_sender_id ON transfers(sender_id);
CREATE INDEX idx_transfers_recipient_id ON transfers(recipient_id);

-- Down migration
-- DROP TABLE transfers;

CREATE TABLE IF NOT EXISTS refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token VARCHAR(255) NOT NULL UNIQUE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_user FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE INDEX idx_refresh_tokens_user ON refresh_tokens(user_id);
CREATE INDEX idx_users_email ON users(email);


-- Create function to delete expired tokens
CREATE OR REPLACE FUNCTION delete_expired_tokens() RETURNS TRIGGER AS $$
BEGIN
    DELETE FROM refresh_tokens WHERE expires_at <= CURRENT_TIMESTAMP;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to call the function
CREATE TRIGGER clear_expired_tokens
AFTER INSERT OR UPDATE ON refresh_tokens
FOR EACH STATEMENT
EXECUTE FUNCTION delete_expired_tokens();
DO $$
BEGIN
    IF to_regclass('public.motivation') IS NULL
       AND to_regclass('public.goal') IS NOT NULL THEN
        ALTER TABLE goal RENAME TO motivation;
    END IF;
END
$$;

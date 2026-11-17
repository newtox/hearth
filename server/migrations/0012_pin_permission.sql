UPDATE roles SET permissions = permissions | 2097152 WHERE permissions & 4 <> 0;
UPDATE overwrites SET allow = allow | 2097152 WHERE allow & 4 <> 0;
UPDATE overwrites SET deny = deny | 2097152 WHERE deny & 4 <> 0;

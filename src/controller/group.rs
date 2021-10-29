use std::collections::HashSet;

use crate::{database::Database, models::Group, webapi::GroupMsg, Error};

/**
 * If the user given by `user_id` has the necessary rights to view the group given by
 * `group_id`, the latter `Group` is returned.
 * If the Group given by 'group_id' does not exist, `Error::NoSuchTarget` is returned. If the
 * user given by `user_id` does not have the necessary rights, to view the group given by
 * `group_id`, `Error::MissingAuthorization` is retuned.
 */
pub fn get_group_info(group_id: u64, user_id: u64, db: &Database) -> Result<Group, Error> {
    // Check, if the querying user is allowed to view the queried user:
    let group = db.get_group(group_id)?.ok_or(Error::NoSuchTarget)?;

    if group.contains_user(user_id) || group.contains_admin(user_id) {
        Ok(group)
    } else {
        Err(Error::MissingAuthorization)
    }
}

/**
 * If the user given by `user_id` has the rights necessary to add a new group, a new group
 * with the attributes given by `group_infos` is added to the DB.
 * If some necessary attributes are missing, `Error::BadCall` is returned. If a group with the
 * given name allready exists in the DB, `Error::TargetExists` is retuned. If `group_infos.id` does
 * not equal `None`, `Error::BadCall` is returned. If the user given by `user_id` does not have the
 * rights necessary to add a new group, `Error::MissingAuthorization` is retuned. If the given
 * Groups number of members or admins exceeds the maximum, that can be stored in the DB
 * (`u16::MAX`), `Error::BadCall` is returned.
 * Otherwise the new Group is written to the DB and returned. The ID of the given Group will be
 * set to a new random and unique value.
 */
pub fn add_group(group_infos: GroupMsg, user_id: u64, db: &Database) -> Result<Group, Error> {
    // Make sure the acting user has the necessary permissions:
    if user_id != 0 {
        // TODO: Implement permissions
        return Err(Error::MissingAuthorization);
    }

    // Make sure all necessary fields exists:
    if group_infos.name.is_none() {
        return Err(Error::BadCall);
    }
    // Make sure the id is not set:
    if group_infos.id.is_some() {
        return Err(Error::BadCall);
    }

    let mut new_group = Group::from(group_infos);
    new_group.admin_ids.push(user_id);

    db.insert_new_group(&mut new_group)?;
    Ok(new_group)
}

/**
 * If the user given by `user_id` has the rights necessary to change the attributes of the
 * group given by `group_infos.id`, these attrbutes will be updated to the values given by
 * `group_infos` and the changes will be written to the DB.
 * If `group_infos.id` equals `None`, `Error::BadCall` is returned.
 * If there is no group with the given ID in the DB, `Error::NoSuchTarget` is retuned.
 * If the given name allready exists in the DB, `Error::TargetExists` is retuned.
 * If the user given by `user_id` does not have the rights necessary to change the given group,
 * `Error::MissingAuthorization` is retuned.
 */
pub fn update_group_infos(
    group_infos: GroupMsg,
    user_id: u64,
    db: &Database,
) -> Result<Group, Error> {
    // Make sure, that the ID is given, the group exists and the acting user has the necessary permissions:
    let mut group = db
        .get_group(group_infos.id.ok_or(Error::BadCall)?.as_int())?
        .ok_or(Error::NoSuchTarget)?;
    if !group.contains_admin(user_id) {
        return Err(Error::MissingAuthorization);
    }

    group_infos.apply_changes(&mut group);

    db.insert_group(&group)?;
    Ok(group)
}

/**
 * If the user given by `user_id` has the rights necessary to add admins to the
 * group given by `group_id`, the users given by `admin_ids` will become admins of the given group
 * and these changes will be written to the DB.
 * If there is no group with the given ID in the DB, `Error::NoSuchTarget` is retuned.
 * If `admin_ids` contains an ID, that does not correspond to a user in the DB,
 * `Error::NoSuchTarget` is retuned.
 * If the user given by `user_id` does not have the rights necessary to add admins to the given
 * group, `Error::MissingAuthorization` is retuned.
 * This function is not thread-save. If a user is removed or added as an admin or member of the
 * given group while this funciton is called, the removal or addition may be revoced.
 */
pub fn add_admin_ids(
    group_id: u64,
    admin_ids: Vec<u64>,
    user_id: u64,
    db: &Database,
) -> Result<Group, Error> {
    // Make sure, that the group exists and the acting user has the necessary permissions:
    let mut group = db.get_group(group_id)?.ok_or(Error::NoSuchTarget)?;
    if !group.contains_admin(user_id) {
        return Err(Error::MissingAuthorization);
    }

    // Make sure we don't add a user multiple times:
    let mut unique_user_ids = HashSet::with_capacity(admin_ids.len());
    for id in admin_ids {
        if !unique_user_ids.contains(&id) && !group.contains_admin(id) {
            unique_user_ids.insert(id);
            group.admin_ids.push(id);
        }
    }

    db.insert_group(&group)?;
    Ok(group)
}

/**
 * If the user given by `user_id` has the rights necessary to add members to the
 * group given by `group_id`, the users given by `member_ids` will become member of the given group
 * and these changes will be written to the DB.
 * If there is no group with the given ID in the DB, `Error::NoSuchTarget` is retuned.
 * If `member_ids` contains an ID, that does not correspond to a user in the DB,
 * `Error::NoSuchTarget` is retuned.
 * If the user given by `user_id` does not have the rights necessary to add members to the given
 * group, `Error::MissingAuthorization` is retuned.
 * This function is not thread-save. If a user is removed or added as an admin or member of the
 * given group while this funciton is called, the removal or addition may be revoced.
 */
pub fn add_member_ids(
    group_id: u64,
    member_ids: Vec<u64>,
    user_id: u64,
    db: &Database,
) -> Result<Group, Error> {
    // Make sure, that the group exists and the acting user has the necessary permissions:
    let mut group = db.get_group(group_id)?.ok_or(Error::NoSuchTarget)?;
    if !group.contains_admin(user_id) {
        return Err(Error::MissingAuthorization);
    }

    // Make sure we don't add a user multiple times:
    let mut unique_user_ids = HashSet::with_capacity(member_ids.len());
    for id in member_ids {
        if !unique_user_ids.contains(&id) && !group.contains_user(id) {
            unique_user_ids.insert(id);
            group.member_ids.push(id);
        }
    }

    db.insert_group(&group)?;
    Ok(group)
}

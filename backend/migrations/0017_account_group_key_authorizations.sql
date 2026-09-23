-- 账号查看与操作授权独立于路由分组绑定
-- 关系存在表示授予整组账号的完整身份查询及约定的账号操作能力
-- 不从既有路由关系回填 避免升级自动扩大 Key 权限
create table account_group_key_authorizations (
  account_group_id text not null,
  client_api_key_id text not null,
  created_at timestamptz not null,
  primary key (account_group_id, client_api_key_id),
  constraint account_group_key_authorizations_group_fk foreign key (account_group_id)
    references account_groups (id)
    on update restrict
    on delete cascade,
  constraint account_group_key_authorizations_key_fk foreign key (client_api_key_id)
    references client_api_keys (id)
    on update restrict
    on delete cascade
);

-- Key 查询反向定位授权分组 账号成员与启用状态在查询时读取当前事实
create index account_group_key_authorizations_key_idx
  on account_group_key_authorizations (client_api_key_id, account_group_id);

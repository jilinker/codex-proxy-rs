<script setup lang="ts">
import type { AccountGroup, ApiKey } from '@/api'
import { Search } from '@lucide/vue'
import { computed, shallowRef, watch } from 'vue'
import { getApiKeys } from '@/api'
import { getGroupKeyAuthorizations, replaceGroupKeyAuthorizations } from '@/api/modules/account-groups'
import BaseButton from '@/components/base/BaseButton.vue'
import BaseCheckbox from '@/components/base/BaseCheckbox.vue'
import BaseEmpty from '@/components/base/BaseEmpty.vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseModal from '@/components/base/BaseModal/index.vue'
import { defineTableColumns } from '@/components/base/BaseTable/columns'
import BaseTable from '@/components/base/BaseTable/index.vue'
import { toast } from '@/components/base/BaseToast'
import { useRequestState } from '@/composables/useRequestState'

const props = defineProps<{ group: AccountGroup | null }>()
const emit = defineEmits<{ close: [] }>()
const open = computed({ get: () => props.group !== null, set: (value) => {
  if (!value)
    emit('close')
} })
const keys = shallowRef<ApiKey[]>([])
const selected = shallowRef(new Set<string>())
const search = shallowRef('')
const saving = shallowRef(false)
const request = useRequestState()
const { loading, error } = request
const rows = computed(() => keys.value.filter(key => `${key.name} ${key.prefix}`.toLowerCase().includes(search.value.trim().toLowerCase())))
const columns = defineTableColumns<ApiKey>([
  { key: 'selection', kind: 'selection' },
  { key: 'name', label: 'Key 名称', kind: 'identity' },
  { key: 'prefix', label: 'Key 前缀', kind: 'custom' },
  { key: 'enabled', label: '状态', kind: 'status' },
])
function toggle(id: string) {
  const next = new Set(selected.value)
  if (next.has(id))
    next.delete(id)
  else next.add(id)
  selected.value = next
}
async function load() {
  const group = props.group
  if (!group)
    return
  const version = request.start()
  keys.value = []
  selected.value = new Set()
  try {
    const ids = await getGroupKeyAuthorizations(group.id, { signal: request.signal })
    const items: ApiKey[] = []
    let cursor: string | undefined
    do {
      const page = await getApiKeys({ limit: 200, cursor }, { signal: request.signal })
      items.push(...page.items.filter(key => key.groups.some(item => item.id === group.id)))
      cursor = page.nextCursor ?? undefined
    } while (cursor && request.isCurrent(version))
    if (request.isCurrent(version)) {
      keys.value = items
      const authorizedIds = new Set(ids)
      selected.value = new Set(items.filter(key => authorizedIds.has(key.id)).map(key => key.id))
    }
  }
  catch (cause) { request.fail(version, cause) }
  finally { request.finish(version) }
}
async function save() {
  if (!props.group || loading.value || saving.value || error.value)
    return
  saving.value = true
  try {
    await replaceGroupKeyAuthorizations(props.group.id, [...selected.value])
    toast.success('Key 授权已更新')
    emit('close')
  }
  catch { /* 请求层展示保存失败 */ }
  finally { saving.value = false }
}
watch(() => props.group?.id, () => {
  request.invalidate()
  keys.value = []
  selected.value = new Set()
  search.value = ''
  request.error.value = ''
  if (props.group)
    void load()
})
</script>

<template>
  <BaseModal v-model="open" title="Key 授权" size="lg" :dismissible="!saving">
    <div class="grid min-w-0 gap-4">
      <div class="grid gap-1">
        <strong class="text-cp text-cp-text">{{ group?.name }}</strong>
        <p class="m-0 text-cp-sm text-cp-text-secondary">
          授权后可查看组内账号完整信息，并使用个人信息、额度重置、刷新及预测
        </p>
      </div>
      <BaseInput v-model="search" placeholder="搜索 Key 名称或前缀" :disabled="loading || saving">
        <template #prefix>
          <Search class="size-4" />
        </template>
      </BaseInput>
      <BaseEmpty v-if="error" title="授权信息加载失败" :description="error">
        <template #action>
          <BaseButton variant="secondary" @click="load">
            重试
          </BaseButton>
        </template>
      </BaseEmpty>
      <BaseTable v-else class="max-h-80 min-h-40" :columns="columns" :rows="rows" :loading="loading" empty-text="暂无匹配的 Key">
        <template #selection="{ row }">
          <BaseCheckbox :model-value="selected.has(row.id)" :disabled="saving" :label="`授权 ${row.name}`" @update:model-value="toggle(row.id)" />
        </template>
        <template #name="{ row }">
          <span class="font-semibold text-cp-text">{{ row.name }}</span>
        </template>
        <template #prefix="{ row }">
          <span class="font-mono text-cp-sm text-cp-text-secondary">{{ row.prefix }}</span>
        </template>
        <template #enabled="{ row }">
          <span :class="row.enabled ? 'text-cp-success' : 'text-cp-text-quaternary'">{{ row.enabled ? '已启用' : '已禁用' }}</span>
        </template>
      </BaseTable>
      <span class="text-cp-sm text-cp-text-secondary" role="status">已选择 {{ selected.size }} 个 Key</span>
    </div>
    <template #footer>
      <BaseButton variant="secondary" :disabled="saving" @click="emit('close')">
        取消
      </BaseButton>
      <BaseButton variant="primary" :loading="saving" :disabled="loading || !!error" @click="save">
        保存授权
      </BaseButton>
    </template>
  </BaseModal>
</template>

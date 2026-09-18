<script setup lang="ts">
import { Search } from '@lucide/vue'
import { computed, shallowRef } from 'vue'
import BaseInput from '@/components/base/BaseInput.vue'
import BaseScrollbar from '@/components/base/BaseScrollbar.vue'
import BaseSegmented from '@/components/base/BaseSegmented.vue'
import RequestHealthTimelineCard from '@/views/dashboard/components/RequestHealthTimelineCard.vue'
import KeyUsageAccountDetail from './components/KeyUsageAccountDetail.vue'
import KeyUsageAccounts from './components/KeyUsageAccounts.vue'
import KeyUsageBudget from './components/KeyUsageBudget.vue'
import KeyUsageHeader from './components/KeyUsageHeader.vue'
import KeyUsageRecords from './components/KeyUsageRecords.vue'
import KeyUsageSkeleton from './components/KeyUsageSkeleton.vue'
import KeyUsageSummary from './components/KeyUsageSummary.vue'
import KeyUsageTrend from './components/KeyUsageTrend.vue'
import { useKeyUsage } from './composables/useKeyUsage'
import { useKeyUsageAccounts } from './composables/useKeyUsageAccounts'
import { keyUsageTime } from './utils/format'

const activeTab = shallowRef('stats')
const statsActive = computed(() => activeTab.value === 'stats')
const accountsActive = computed(() => activeTab.value === 'accounts')
const { period, model, kind, refreshInterval, overview, overviewLoading, overviewError, refreshing, recordsStale, records, refresh, changePageSize, changePage } = useKeyUsage(statsActive)
const { items, currentPage, pageSize, total, loading: recordsLoading, error: recordsError } = records
const accountUsage = useKeyUsageAccounts(accountsActive)
const { items: accountItems, currentPage: accountPage, pageSize: accountPageSize, total: accountTotal, loading: accountsLoading, error: accountsError } = accountUsage.accounts

function refreshActiveTab() {
  if (statsActive.value)
    void refresh()
  else
    void accountUsage.refresh()
}
</script>

<template>
  <main class="h-dvh overflow-hidden bg-cp-bg-layout text-cp-text">
    <BaseScrollbar>
      <div class="mx-auto flex min-h-full w-full max-w-480 flex-col gap-5 p-4 min-[961px]:p-6">
        <KeyUsageHeader v-model:period="period" v-model:refresh-interval="refreshInterval" :name="overview?.key.name" :prefix="overview?.key.prefix" :refreshing="statsActive ? refreshing || overviewLoading : accountsLoading" :show-stats-controls="statsActive" @refresh="refreshActiveTab" />
        <BaseSegmented v-model="activeTab" class="self-start" label="Key 用量视图" :options="[{ label: '使用统计', value: 'stats' }, { label: '账号用量', value: 'accounts' }]" />
        <div v-if="statsActive" class="flex flex-wrap items-center justify-between gap-3">
          <span v-if="overview" class="text-cp-sm text-cp-text-tertiary">更新于 {{ keyUsageTime(overview.asOf).slice(11) }}</span>
          <BaseInput v-model="model" class="ml-auto w-60 max-w-full" placeholder="输入完整模型名称" aria-label="筛选统计和日志的模型" :maxlength="128">
            <template #prefix>
              <Search class="size-4" />
            </template>
          </BaseInput>
        </div>
        <p v-if="statsActive && overviewError" role="alert" class="m-0 rounded-cp-lg bg-cp-error-container px-4 py-3 text-cp-sm text-cp-error-text">
          {{ overviewError }}{{ overview ? '，暂时保留上次结果。' : '，请点击顶部刷新重试。' }}
        </p>
        <template v-if="statsActive && overview">
          <KeyUsageSummary :summary="overview.summary" />
          <div class="grid min-w-0 gap-5 xl:grid-cols-[minmax(0,1.4fr)_minmax(400px,1fr)]">
            <KeyUsageTrend class="min-w-0" :points="overview.trend" />
            <KeyUsageBudget :budget="overview.key" />
          </div>
          <RequestHealthTimelineCard :timeline="overview.healthTimeline" />
        </template>
        <KeyUsageSkeleton v-else-if="statsActive && overviewLoading" />
        <KeyUsageRecords v-if="statsActive" v-model:kind="kind" :rows="items" :pagination="{ currentPage, pageSize, total }" :loading="recordsLoading" :error="recordsError" :stale="recordsStale" @page-change="changePage" @page-size-change="changePageSize" />
        <KeyUsageAccounts v-else :rows="accountItems" :pagination="{ currentPage: accountPage, pageSize: accountPageSize, total: accountTotal }" :loading="accountsLoading" :error="accountsError" :scope-state="accountUsage.scopeState.value" @detail="accountUsage.openDetail" @page-change="accountUsage.changePage" @page-size-change="accountUsage.changePageSize" />
      </div>
    </BaseScrollbar>
    <KeyUsageAccountDetail v-model="accountUsage.detailOpen.value" :account="accountUsage.detail.value" :loading="accountUsage.detailLoading.value" :error="accountUsage.detailError.value" />
  </main>
</template>

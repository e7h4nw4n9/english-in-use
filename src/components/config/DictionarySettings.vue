<script setup lang="ts">
defineProps<{
  saveResultsOffline: boolean
  searchResultLimit: number
}>()

const emit = defineEmits<{
  (event: 'update:saveResultsOffline', value: boolean): void
  (event: 'update:searchResultLimit', value: number): void
}>()
</script>

<template>
  <div class="dictionary-settings">
    <a-form
      layout="horizontal"
      :label-col="{ xs: { span: 24 }, sm: { span: 6 } }"
      :wrapper-col="{ xs: { span: 24 }, sm: { span: 18 } }"
      label-align="right"
    >
      <a-form-item label="查询候选词个数">
        <a-input-number
          :value="searchResultLimit"
          :min="1"
          :max="15"
          style="width: 100%"
          @update:value="emit('update:searchResultLimit', $event ?? 5)"
        />
      </a-form-item>

      <a-form-item label="离线保存查询结果">
        <a-switch
          :checked="saveResultsOffline"
          @update:checked="emit('update:saveResultsOffline', $event)"
        />
        <p class="setting-description">
          开启后保存查询详情，并在首次播放时保存对应音频；关闭不会删除已有缓存。
        </p>
      </a-form-item>
    </a-form>
  </div>
</template>

<style scoped>
.dictionary-settings {
  max-width: 720px;
}
.setting-description {
  margin: 8px 0 0;
  color: var(--ant-color-text-secondary);
  font-size: 13px;
}
</style>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import { FormField, SelectField, TextArea, TextInput } from '@/components/form'
import {
  createItem,
  ITEM_TYPE_LABELS,
  ITEM_TYPES,
  SPACE_LABELS,
  SPACES,
  type Item,
  type ItemType,
  type Space,
} from '@/apis/items'

const emit = defineEmits<{
  created: [item: Item]
}>()

const open = ref(false)
const submitting = ref(false)
const errorMessage = ref('')

const title = ref('')
const type = ref<ItemType>('idea')
const space = ref<Space>('personal')
const body = ref('')

const typeOptions = ITEM_TYPES.map((value) => ({ value, label: ITEM_TYPE_LABELS[value] }))
const spaceOptions = SPACES.map((value) => ({ value, label: SPACE_LABELS[value] }))

watch(open, (value) => {
  if (!value) reset()
})

function reset() {
  title.value = ''
  type.value = 'idea'
  space.value = 'personal'
  body.value = ''
  errorMessage.value = ''
}

function selectType(value: string) {
  type.value = value as ItemType
}

function selectSpace(value: string) {
  space.value = value as Space
}

async function submit() {
  if (submitting.value || !title.value.trim()) return

  submitting.value = true
  errorMessage.value = ''
  try {
    const item = await createItem({
      title: title.value,
      type: type.value,
      space: space.value,
      body: body.value,
    })
    emit('created', item)
    open.value = false
  } catch (error) {
    errorMessage.value = error instanceof Error ? error.message : String(error)
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <Dialog v-model:open="open">
    <DialogTrigger as-child>
      <Button>新建条目</Button>
    </DialogTrigger>
    <DialogContent>
      <DialogHeader>
        <DialogTitle>新建条目</DialogTitle>
        <DialogDescription>写入 vault 的 Markdown 文件，保存后立即出现在列表中。</DialogDescription>
      </DialogHeader>

      <form class="space-y-4" novalidate @submit.prevent="submit">
        <FormField label="标题" required>
          <template #default="{ id, invalid, describedBy, required }">
            <TextInput
              :id="id"
              v-model="title"
              :invalid="invalid"
              :described-by="describedBy"
              :required="required"
              placeholder="一句话说清要做什么"
            />
          </template>
        </FormField>

        <div class="grid gap-4 sm:grid-cols-2">
          <FormField label="类型">
            <template #default="{ id, invalid, describedBy }">
              <SelectField
                :id="id"
                :model-value="type"
                :options="typeOptions"
                :invalid="invalid"
                :described-by="describedBy"
                :clearable="false"
                @update:model-value="selectType"
              />
            </template>
          </FormField>

          <FormField label="空间">
            <template #default="{ id, invalid, describedBy }">
              <SelectField
                :id="id"
                :model-value="space"
                :options="spaceOptions"
                :invalid="invalid"
                :described-by="describedBy"
                :clearable="false"
                @update:model-value="selectSpace"
              />
            </template>
          </FormField>
        </div>

        <FormField label="正文" hint="Markdown，可留空">
          <template #default="{ id, describedBy }">
            <TextArea
              :id="id"
              v-model="body"
              :rows="6"
              :described-by="describedBy"
              placeholder="背景、验收标准……"
            />
          </template>
        </FormField>

        <p v-if="errorMessage" class="text-destructive text-sm" role="alert">{{ errorMessage }}</p>

        <DialogFooter>
          <DialogClose as-child>
            <Button type="button" variant="outline">取消</Button>
          </DialogClose>
          <Button type="submit" :disabled="submitting || !title.trim()">
            {{ submitting ? '创建中…' : '创建' }}
          </Button>
        </DialogFooter>
      </form>
    </DialogContent>
  </Dialog>
</template>

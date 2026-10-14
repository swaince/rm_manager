import { globalIgnores } from 'eslint/config'
import { defineConfigWithVueTs, vueTsConfigs } from '@vue/eslint-config-typescript'
import pluginVue from 'eslint-plugin-vue'
import pluginOxlint from 'eslint-plugin-oxlint'
import skipFormatting from 'eslint-config-prettier/flat'

// To allow more languages other than `ts` in `.vue` files, uncomment the following lines:
// import { configureVueProject } from '@vue/eslint-config-typescript'
// configureVueProject({ scriptLangs: ['ts', 'tsx'] })
// More info at https://github.com/vuejs/eslint-config-typescript/#advanced-setup

export default defineConfigWithVueTs(
  {
    name: 'app/files-to-lint',
    files: ['**/*.{vue,ts,mts,tsx}'],
  },

  globalIgnores([
    '**/{node_modules,src-tauri,dist,dist-ssr,coverage}/**',
    'src/components/ui/**',
    'src/components/ai-elements/**',
  ]),

  ...pluginVue.configs['flat/essential'],
  vueTsConfigs.recommended,

  // shortcut: 仅检查模板中的显式标签与类型；引入别名或渲染函数时扩展规则和回归测试。
  {
    name: 'app/form-controls',
    files: ['**/*.vue'],
    rules: {
      'vue/no-restricted-html-elements': [
        'error',
        {
          element: ['input', 'select', 'textarea', 'button', 'option', 'datalist'],
          message: '表单控件必须使用 UI 组件库或已有封装，禁止直接使用原生控件。',
        },
      ],
      'vue/no-restricted-syntax': [
        'error',
        {
          selector:
            'VElement[rawName="Input"] > VStartTag > ' +
            'VAttribute[directive=false][key.name="type"]' +
            '[value.value=/^(date|time|datetime-local|month|week)$/i]',
          message: '日期和时间必须使用选择器组件或统一封装，禁止 Input 退回浏览器原生控件。',
        },
        {
          selector:
            'VElement[rawName="Input"] > VStartTag > ' +
            'VAttribute[directive=true][key.name.name="bind"][key.argument.name="type"] ' +
            ':matches(VExpressionContainer > Literal, ' +
            'VExpressionContainer > ConditionalExpression > Literal.consequent, ' +
            'VExpressionContainer > ConditionalExpression > Literal.alternate)' +
            '[value=/^(date|time|datetime-local|month|week)$/i]',
          message: '日期和时间必须使用选择器组件或统一封装，禁止 Input 退回浏览器原生控件。',
        },
        {
          selector:
            'VElement[rawName="Input"] > VStartTag > ' +
            'VAttribute[directive=true][key.name.name="bind"][key.argument=null] > ' +
            'VExpressionContainer > ObjectExpression > ' +
            'Property:matches([key.name="type"], [key.value="type"]) > ' +
            'Literal.value[value=/^(date|time|datetime-local|month|week)$/i]',
          message: '日期和时间必须使用选择器组件或统一封装，禁止 Input 退回浏览器原生控件。',
        },
        {
          selector:
            'VElement[rawName="component"] > VStartTag > ' +
            'VAttribute[directive=false][key.name="is"]' +
            '[value.value=/^(input|select|textarea|button|option|datalist)$/i]',
          message: '动态组件禁止指向原生表单控件，请使用 UI 组件库或已有封装。',
        },
        {
          selector:
            'VElement[rawName="component"] > VStartTag > ' +
            'VAttribute[directive=true][key.name.name="bind"][key.argument.name="is"] ' +
            ':matches(VExpressionContainer > Literal, ' +
            'VExpressionContainer > ConditionalExpression > Literal.consequent, ' +
            'VExpressionContainer > ConditionalExpression > Literal.alternate)' +
            '[value=/^(input|select|textarea|button|option|datalist)$/i]',
          message: '动态组件禁止指向原生表单控件，请使用 UI 组件库或已有封装。',
        },
      ],
    },
  },

  ...pluginOxlint.buildFromOxlintConfigFile('.oxlintrc.json'),

  skipFormatting,
)

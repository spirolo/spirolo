// The home page in Simplified Chinese, translated from en.ts.
import type { LandingCopy } from './types';

export const zh: LandingCopy = {
  lang: 'zh-CN',
  numbers: 'zh-CN',
  meta: {
    title: '{product}：测试套件缺少的那一层检查',
    description:
      '测试套件通过，只能说明测试运行过。{product} 读取整个测试套件，连同初始化文件和 CI，找出哪些测试即使通过也不能证明什么。它从不运行测试。',
  },
  nav: { docs: '文档', language: { text: 'English', url: '/' } },
  footer: { licence: 'Apache 2.0 许可证', docs: '文档', rules: '规则', source: '源码' },
  install: { copy: '复制', copied: '已复制', copyCommand: '复制 {command}' },
  stickers: {
    bugInAJar: '紫色千足虫吉祥物 Spirolo 飞奔而过，带着一个罐子，里面关着一只气鼓鼓的甲虫。',
    cleanRun: '紫色千足虫吉祥物 Spirolo 在通过的测试卡片堆成的小山上插好旗，双臂交叉。',
  },
  hero: {
    eyebrow: '提示级 alpha 版本，面向 JavaScript 和 TypeScript 测试套件',
    title: '测试套件缺少的\u200b那一层检查。',
    lead: '测试套件通过，只能说明测试运行过。{product} 读取整个测试套件，连同初始化文件和 CI，找出哪些测试即使通过也不能证明什么。它从不运行测试。',
    link: '安装与首次运行（英文）',
  },
  replay: {
    findings: '发现项',
    total: '在 {repos} 个开源项目中检出 {findings} 条发现项。',
    read: '读取 {tests} 个测试，用时约 {seconds} 秒。',
    kinds: {
      'absence-anchor': '弱测试',
      'fail-closed-setup': '缺失的防护',
      'dead-skip': '无人跟进的跳过',
      'duplicate-test': '重复测试',
    },
    table: { project: '项目', tests: '测试数', findings: '发现项', readIn: '读取用时' },
    caption:
      '按实际速度回放：在同一台 Linux 机器上，针对固定的提交版本，将 alpha 版本从启动到退出完整运行七次，取耗时中位数。发现项为提示级，并非已确认的缺陷。',
  },
  value: {
    title: '智能体写测试的速度，\u200b已经超过了\u200b人工审查的速度',
    intro:
      '智能体一下午就能添加数百个测试，而且全部通过。审查者看不完，CI 也只报告测试已通过。验证必须跟上测试数量的增长。',
    points: [
      {
        lead: '速度快',
        title: '跟得上测试数量的增长',
        text: '用 Rust 编写。它读取了 {repos} 个项目中的 {tests} 个测试，速度足以让每个智能体在每次修改后运行检查。',
      },
      {
        lead: '运行 0 个测试',
        title: '不给 CI 增加测试运行负担',
        text: '它只读取文件，不需要浏览器、数据库，也不需要再跑一遍测试。它找出的 {duplicates} 个重复测试，让 CI 把同一份检查做了两遍。',
      },
      {
        lead: '5 个 MCP 工具',
        title: '让智能体检查自己写的测试',
        text: '智能体可以边写边调用同样的检查，在人工审查前修好弱测试。',
      },
      {
        lead: '{guards} 处缺失的防护',
        title: '让测试通过重新有意义',
        text: '初始化防护缺失时，控制台错误或泄漏的 mock 不会让测试失败，该失败的测试也可能通过。当没人能逐个审查测试时，合并门禁就必须守得住。',
      },
      {
        lead: '只针对新增发现项',
        title: '守住现有标准',
        text: '运行一次 ratchet，CI 就只因新增发现项而失败。不必先集中清理历史问题，之后每批新增测试都要满足同一标准。',
      },
      {
        lead: '结果确定',
        title: '来自 LLM 之外的独立检查',
        text: '同样的代码，每次都得到同样的报告。不用模型，也不联网：代码留在你的机器上，检查测试的工具也与编写测试的工具属于不同类型。',
      },
    ],
  },
  families: {
    title: '测试全部通过，\u200b也回答不了\u200b这五个问题',
    intro: '每条规则的评分都公开，全部 {count} 条规则见',
    link: '规则页面（英文）',
    after: '。',
    tiles: {
      audit: {
        title: '弱测试',
        question: '这个测试能发现代码中的错误吗？',
        strong: '发现代码中的错误',
        stat: '{count} 个弱测试',
      },
      gate: {
        title: '合并门禁',
        question: '测试失败，真的能阻止合并吗？',
        strong: '真的能阻止合并',
        stat: '{count} 处缺失的防护',
      },
      hygiene: {
        title: '未发挥作用的测试',
        question: '哪些测试没有增加任何检查，只是重复了套件已有的检查？',
        strong: '没有增加任何检查',
        stat: '{count} 个重复测试',
      },
      flake: {
        title: '不稳定测试',
        question: '哪些测试可能这次通过，下次就失败？',
        strong: '这次通过，下次就失败',
        stat: '实验性',
      },
      impact: {
        title: '健康状况',
        question: '在哪里优先补充测试最有价值？',
        strong: '优先补充测试最有价值',
        stat: '实验性',
      },
    },
  },
  health: {
    title: '用一个分数\u200b观察变化，\u200b不作为门禁依据',
    text: '每份报告开头都有一个暂定的健康评分，满分 100 分。每个规则类别按其检查对象中存在发现项的比例扣分，并设有各自的扣分上限；默认视图之外的规则按一半权重计入。这个分数用于持续观察整体变化：不参与任何门禁，面向机器的输出格式也不包含它。',
    link: '阅读报告（英文）',
    gradeLabel: '{grade} 级',
    outOf: '满分 100',
  },
  layers: {
    title: '测试通过，\u200b不等于测到了问题',
    intro:
      '一个通过的测试套件依赖四层检查。多数团队只检查了第一层，就以为其余几层也有保障。其中两层没有工具检查，它们共同构成了缺少的那一层检查，也是 {product} 关注的地方。',
    checkedBy: '由 {product} 检查',
    layers: [
      {
        name: '测试文件内的错误',
        job: '没有断言的测试、被设为单独运行的测试、未等待完成的查询。',
        who: '由 ESLint 测试插件检查',
      },
      {
        name: '每个测试依赖的初始化环境',
        job: '控制台错误、泄漏的 mock 或未匹配的请求，会让测试失败，还是被悄悄放过。',
        who: '没有其他工具检查',
      },
      {
        name: '断言能否捕获缺陷',
        job: '变异测试会注入小缺陷，再运行测试套件，看测试能否发现。检查彻底，但大型套件运行起来很慢：每注入一个缺陷，就要运行一次。',
        who: '由 Stryker 检查',
      },
      {
        name: '测试与代码之间的关联',
        job: '匹配不到真实导出的 mock、未证明渲染已发生就检查元素不存在的断言、无人跟进的跳过、从不拦截的门禁。',
        who: '没有其他工具检查',
      },
    ],
  },
  defaultView: {
    title: '目前处于 alpha 阶段，\u200b更多规则\u200b即将加入。',
    intro:
      '目前还没有规则达到已验证级别，因此默认视图保持谨慎：只有当一条规则的抽样发现项中，至少 15 条已完成判定，且其中至少 80% 属实，才会显示。其他规则仍会运行，使用 --all 即可查看，并附有级别标注。',
    link: '规则如何评估（英文）',
    after: '。',
    viewLabel: '视图',
    tierHeading: {
      proven: '已验证（proven）：评估结论为 PASS',
      advisory: '提示级（advisory）：至少 15 条已判定，其中至少 80% 属实',
      experimental: '实验性（experimental）：其余评估结论不为 FAIL 或 DROPPED 的规则',
      hidden: '隐藏（hidden）：评估结论为 FAIL 或 DROPPED',
    },
    tierName: { proven: '已验证', advisory: '提示级', experimental: '实验性', hidden: '隐藏' },
    none: '无',
    folded: '{count} 条规则已运行但未显示：{groups}。运行 ',
    foldedGroup: '{count} 条{tier}',
    foldedJoin: '、',
    foldedTail: ' 可查看，并附有各自的级别标注。',
  },
  questions: {
    title: '常见问题',
    faqs: [
      {
        question: '{product} 是什么？',
        answer:
          '{product} 读取 JavaScript 或 TypeScript 测试套件，连同配置、初始化文件、mock 和 CI，找出那些通过了却不能证明什么的测试。它是通过 npm 分发的原生二进制程序，采用 Apache 2.0 许可证，目前处于提示级 alpha 阶段。',
      },
      {
        question: '它会运行我的测试吗？',
        answer:
          '不会。它只读取文件，从不执行测试，因此不需要浏览器、数据库或网络，只需先安装项目依赖，以便解析导入。在 Linux x64 上，它用 {seconds} 秒读取了 {repos} 个项目中的 {tests} 个测试。',
      },
      {
        question: '它与 ESLint 测试插件有什么区别？',
        answer:
          'ESLint 一次检查一个文件，而测试套件造成虚假通过的原因大多藏在文件之间：吞掉控制台错误的初始化逻辑、匹配不到任何导出的 mock、无人跟进的跳过。继续用 ESLint 检查文件内部；Spirolo 读取文件之间的关联。',
      },
      {
        question: '它与变异测试有什么区别？',
        answer:
          'Stryker 注入小缺陷，每个缺陷都要重新运行一次测试套件，检查彻底，但大型套件运行起来很慢。Spirolo 只读取一遍，指出哪些结构让缺陷漏过检查。它也能读取 Stryker 报告，找出检出相同变异体的测试。',
      },
      {
        question: '它支持哪些测试套件？',
        answer:
          '支持 Jest、Vitest 和 Playwright，以及搭配使用的 Testing Library、MSW 和 nock，运行于 Linux 和 macOS（x64 和 arm64）。规则在 React 项目及纯 Node 或 TypeScript 项目上评估；当前 alpha 版本不涵盖 Cypress、Vue 和 Windows。',
      },
      {
        question: '误报怎么办？',
        answer:
          '只有当一条规则的抽样发现项中，至少 15 条已完成判定，且其中至少 80% 属实，才会默认显示，评分也会公开。反馈误报时附上文件和行号，就能帮助改进规则。',
      },
      {
        question: '它会让构建失败吗？',
        answer:
          '只有你主动配置后才会：默认每条发现项的严重级别都是 info，退出码保持为 0。在 {config} 中将规则的严重级别设为 warn 或 error，或运行一次 ratchet，让 CI 只因新增发现项而失败。',
      },
    ],
  },
  closing: {
    title: '{product} 会在你的\u200b测试套件中\u200b发现什么？',
    text: '测试是上线前的最后一道检查。它在 {repos} 个热门开源项目中检出 {findings} 条发现项，每个项目都只需数秒即可读取。先安装项目依赖，再从项目根目录运行。',
    link: '安装与首次运行（英文）',
  },
};

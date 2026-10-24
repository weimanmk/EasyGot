# JSON标准题库 v1

编码UTF-8；根对象含 `schemaVersion:1`、非空 `title`、`questions` 数组（平铺题库）或 `directory` 目录（多套卷），可选根字段 `id`（题库稳定id）、`materialIds`（整库关联资料id列表）和 `assets`（SHA256→Base64）。建议导出文件作为规范模板。单次上限20000题/30MB；完整备份是另一格式，不能作为题库导入。

| 字段 | 类型 | 规则 |
| --- | --- | --- |
| id | string | 全库稳定唯一，同id导入表示更新 |
| type | string | single / multiple / boolean / short / essay / calculation |
| stem | string | 非空题干 |
| maxScore | number | 大于0且不超过10000 |
| options | array | 选择题必需，2–12项，项为唯一id和非空text |
| answer | array / boolean | 选择题为选项id数组；单选恰好一个，多选至少一个；判断为真正布尔值 |
| referenceAnswer | string | 简答、论述、计算题必需，非空 |
| explanation | string | 已有解析，离线可见 |
| chapter | string | 章节 |
| knowledgePoints | string[] | 知识点；AI按权重组卷时第一项为主知识点 |
| difficulty | integer | 出题时1–5 |
| strictRequirements | string | 评分时必须准确满足的术语、步骤或公式要求 |
| materialIds | string[] | 单题参考教材id，与所在题库关联资料合并使用；合并后为空时检索全部资料 |
| origin | string | original / ai，AI新题必须携带sources数组 |
| sources | array | `{materialId,title,locator}`或`{url,title,locator}`，不编造来源 |
| image | string | 可选题干图片SHA256；实际内容由assets或已有本地素材提供 |

```json
{
  "schemaVersion": 1,
  "title": "基础知识",
  "questions": [
    {
      "id": "chapter1-q1",
      "type": "short",
      "stem": "解释版本冲突的含义。",
      "maxScore": 5,
      "referenceAnswer": "同一记录在不同设备上发生并发修改，产生多个不能直接判断取舍的版本。",
      "explanation": "关键在同一记录与并发修改。",
      "chapter": "同步",
      "knowledgePoints": ["版本冲突"],
      "difficulty": 2,
      "strictRequirements": "必须指出是对同一记录的并发修改。",
      "origin": "original",
      "materialIds": [],
      "sources": []
    }
  ]
}
```

图片JSON：`assets`为`{"64位小写sha256":"base64字节"}`，`image`引用同一个hash。导入校验hash与实际字节匹配；图片缺失则不导入题目。多选答案顺序无关，但重复项、未知选项拒绝；不允许字符串"true"代替判断题布尔值。

完整备份根对象为 `{backupVersion:1,revisions:[...],assets:{...}}`，使用设置中的备份入口导入，以版本合并方式保留冲突，不含凭据。

整页照片识题会保留 `originalNumber` 和 `recognitionPages:[{asset:SHA256}]`。素材随题库导出/备份/同步保留，但不会自动裁切复杂图形；有图形的题目应核对草稿并设置题干 `image`。原图未包含标准答案时不会自动解题补齐，需人工补全后校验入库。

没有id的旧题库按名称保留题库分组；建议导出后使用稳定id。重导入未带materialIds时保留已有整库关联；取消关联请用App内的关联资料设置。题库JSON只携带关联id，教材原文件需另行导入或使用完整备份/WebDAV同步。

## 多套卷与目录

一个JSON可以包含10套或更多套题。`directory`是有序数组；目录节点有稳定唯一的`id`和`title`。文件夹用`children`存放子目录，套卷用`questions`内嵌题目，或用`questionIds`按顺序引用根`questions`题目池。文件夹不能同时放题目。每套可选`timeLimitMinutes`（1–360分钟）。最多8层、1000个目录节点，总题量仍最多20000题。

```json
{
  "schemaVersion": 1,
  "id": "math-10-papers",
  "title": "数学套卷题库",
  "directory": [
    {
      "id": "paper-01",
      "title": "第1套",
      "timeLimitMinutes": 30,
      "questions": [
        {"id":"p01-q01","type":"boolean","stem":"2是偶数。","maxScore":2,"answer":true}
      ]
    },
    {
      "id": "paper-02",
      "title": "第2套",
      "timeLimitMinutes": 45,
      "questions": [
        {"id":"p02-q01","type":"boolean","stem":"3是偶数。","maxScore":2,"answer":false}
      ]
    }
  ]
}
```

完整的10套题示例见[examples/multi-paper-bank.json](../examples/multi-paper-bank.json)。每道内嵌题的id在整个题库中唯一；要共享同一道题时，将其放根`questions`池，再用`questionIds`引用。不存在的题目引用、套卷内重复引用和重复目录id会拒绝导入，不会半途写入。

App中选中题库后显示「套卷目录」，选择一套再练习、考试或导出PDF，保留该套的题序；章节/知识点搜索只作用于当前套卷。未选套卷时范围为整个题库。目录与题库关联资料一同备份、WebDAV同步。平铺旧JSON无需修改。

导出JSON采用根`questions`池＋目录`questionIds`的规范形式，避免重复存题，仍可重新导入。导出某一套或筛选结果时会移除范围外的目录和引用；单套导出使用独立题库id，避免重导入时覆盖原整库目录。导入旧版App可能丢失目录，建议设备统一升级。

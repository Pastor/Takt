package org.lam.intellij.editor

import com.intellij.lang.Commenter

/**
 * Комментирование Lam (задача 0022-03).
 *
 * Строчный комментарий — `//` (Ctrl+/); блочный — `/* … */` (язык его
 * поддерживает, см. лексер). Документационный `///` при построчном
 * комментировании не отличается от `//`.
 */
class LamCommenter : Commenter {
    override fun getLineCommentPrefix(): String = "//"
    override fun getBlockCommentPrefix(): String = "/*"
    override fun getBlockCommentSuffix(): String = "*/"
    override fun getCommentedBlockCommentPrefix(): String? = null
    override fun getCommentedBlockCommentSuffix(): String? = null
}

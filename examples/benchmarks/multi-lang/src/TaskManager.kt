package com.example.tasks

import java.time.Instant

/** Priority levels for tasks. */
enum class Priority { LOW, MEDIUM, HIGH, CRITICAL }

/** A single task item. */
data class Task(
    val id: String,
    val title: String,
    val priority: Priority = Priority.MEDIUM,
    val createdAt: Instant = Instant.now(),
    val completed: Boolean = false,
)

/** Manages a collection of tasks. */
class TaskManager {
    private val tasks = mutableListOf<Task>()

    /** Add a new task. */
    fun add(title: String, priority: Priority = Priority.MEDIUM): Task {
        val task = Task(id = generateId(), title = title, priority = priority)
        tasks.add(task)
        return task
    }

    /** Mark a task as completed. */
    fun complete(id: String): Boolean {
        val idx = tasks.indexOfFirst { it.id == id }
        if (idx < 0) return false
        tasks[idx] = tasks[idx].copy(completed = true)
        return true
    }

    /** List tasks filtered by priority. */
    fun byPriority(priority: Priority): List<Task> =
        tasks.filter { it.priority == priority && !it.completed }

    /** Count incomplete tasks. */
    fun pendingCount(): Int = tasks.count { !it.completed }

    private fun generateId(): String = "task-${tasks.size + 1}"
}
